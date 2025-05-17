import sys
import struct
import argparse
from tqdm import tqdm
from collections import defaultdict

try:
    import numpy as np
    from PIL import ImageFont, ImageDraw, Image
except ImportError as err:
    print(err, "尝试运行 `python -m pip install -r requirements.txt`")
    exit()

__version__ = "1.2"  # 版本更新为支持大哈希表

# --- 优化的 BMF 文件结构 ---
"""
文件头 (24字节):
    [0:3]   魔数 0x0B 0x2D 0x0E
    [3:6]   位图数据起始偏移 (大端序)
    [6]     字号
    [7]     单字符字节数
    [8:11]  哈希表起始偏移 (3B)
    [11:14] 哈希槽数量 (3B)
    [14:24] 保留
"""

def get_im(word, width, height, font, offset: tuple = (0, 0)) -> Image.Image:
    im = Image.new('1', (width, height), (1,))
    draw = ImageDraw.Draw(im)
    draw.text(offset, word, font=font)
    return im
    
def to_bitmap(word: str, font_size: int, font, offset=(0, 0)) -> bytearray:
    """ 获取点阵字节数据"""
    code = 0x00
    data_code = word.encode("utf-8")

    # 获取字节码
    try:
        for byte in range(len(data_code)):
            code |= data_code[byte] << (len(data_code) - byte - 1) * 8
    except IndexError:
        print(word, word.encode("utf-8"))

    # 获取点阵图
    bp = np.pad(
        (~np.asarray(get_im(word, width=font_size, height=font_size, font=font, offset=offset))).astype(np.int32),
        ((0, 0), (0, int(np.ceil(font_size / 8) * 8 - font_size))), 'constant',
        constant_values=(0, 0))
    
    # 点阵映射 MONO_HLSB
    bmf = []
    for line in bp.reshape((-1, 8)):
        v = 0x00
        for _ in line:
            v = (v << 1) + _
        bmf.append(v)
    return bytearray(bmf)
         
def generate_hash_table(words, start_bitmap, bytes_per_char):
    """生成哈希表 (开放寻址法)"""
    table_size = max(2 * len(words), 256)  # 允许超过65535
    hash_table = bytearray(table_size * 6)
    
    for idx, word in enumerate(words):
        unicode = ord(word)
        slot = unicode % table_size
        
        while hash_table[slot*6:slot*6+2] != b'\x00\x00':
            slot = (slot + 1) % table_size
        
        offset = start_bitmap + idx * bytes_per_char
        hash_table[slot*6:slot*6+2] = struct.pack("<H", unicode)
        hash_table[slot*6+2:slot*6+6] = struct.pack("<I", offset)
        
    return hash_table
    

def run(font_file, font_size=16, offset=(0, -2), text_file=None, text=None, bitmap_fonts_name=None):
    font = ImageFont.truetype(font=font_file, size=font_size)
    
    if text:
        words = list(set(text))
    else:
        with open(text_file, "r", encoding="utf-8") as f:
            words = list(set(f.read()))
    words.sort()
    font_num = len(words)
    
    bytes_per_char = int(np.ceil(font_size / 8)) * font_size
    bitmap_fonts_name = bitmap_fonts_name or f"{font_file.split('.')[0]}-{font_num}-{font_size}.bmf"
    
    with open(bitmap_fonts_name, "wb") as f:
        print(f"生成点阵字体 (v{__version__})，字符数: {font_num}")
        
        header = bytearray(24)
        header[0:3] = bytes([0x0B, 0x2D, 0x0E])
        header[6] = font_size
        header[7] = bytes_per_char
        f.write(header)
        
        unicode_data = bytearray()
        for w in tqdm(words, desc="生成Unicode表"):
            unicode_data += struct.pack("<H", ord(w))
        f.write(unicode_data)
        
        start_bitmap = f.tell()
        print(f"位图起始: 0x{start_bitmap:X}")
        
        #bitmaps = bytearray()
        for w in tqdm(words, desc="生成位图"):
            f.write(to_bitmap(w, font_size, font, offset))
        #f.write(bitmaps)
        
        hash_table = generate_hash_table(words, start_bitmap, bytes_per_char)
        hash_start = f.tell()
        f.write(hash_table)
        
        f.seek(3)
        f.write(struct.pack("<I", start_bitmap)[1:4])
        
        f.seek(8)
        f.write(hash_start.to_bytes(3, byteorder='little'))
        hash_slots = len(hash_table) // 6
        f.write(hash_slots.to_bytes(3, byteorder='little'))
        
        print(f"生成完成: {bitmap_fonts_name} (总大小: {f.tell()/1024:.2f}KB)")
        return bitmap_fonts_name

class BMFParser:
    def __init__(self, data):
        self.font_size = data[6]
        self.bytes_per_char = data[7]
        self.bitmap_start = int.from_bytes(data[3:6], "little")
        self.hash_start = int.from_bytes(data[8:11], "little")
        self.hash_slots = int.from_bytes(data[11:14], "little")
        self.data = data
    
    def get_char(self, unicode):
        slot = unicode % self.hash_slots
        while True:
            entry = self.data[self.hash_start + slot*6 : self.hash_start + slot*6 +6]
            entry_unicode = struct.unpack("<H", entry[0:2])[0]
            entry_offset = struct.unpack("<I", entry[2:6])[0]
            
            if entry_unicode == unicode:
                return self.data[entry_offset : entry_offset + self.bytes_per_char]
            elif entry_unicode == 0:
                return None
            slot = (slot + 1) % self.hash_slots
    
    def render_to_console(self, unicode, foreground="█", background=" "):
        """
        将字符渲染到控制台
        :param unicode: 要渲染的字符Unicode码
        :param foreground: 前景字符(默认实心方块)
        :param background: 背景字符(默认空格)
        """
        char_data = self.get_char(unicode)
        if char_data is None:
            print(f"字符U+{unicode:04X}不存在")
            return
        
        bytes_per_line = int(np.ceil(self.font_size / 8))
        for y in range(self.font_size):
            line_byte = char_data[y*bytes_per_line : (y+1)*bytes_per_line]
            bits = []
            for byte in line_byte:
                bits.extend([(byte >> i) & 1 for i in range(7, -1, -1)])
            
            # 只取字体宽度的部分
            line = "".join([foreground if bit else background for bit in bits[:self.font_size]])
            print(line)
    
    def print_char(self, char, foreground="█", background=" "):
        """
        打印指定字符到控制台
        :param char: 要打印的字符(字符串)
        :param foreground: 前景字符(默认实心方块)
        :param background: 背景字符(默认空格)
        """
        if len(char) != 1:
            raise ValueError("只能打印单个字符")
        self.render_to_console(ord(char), foreground, background)
                    

def load_bmf(filename):
    with open(filename, "rb") as f:
        data = f.read()
    return BMFParser(data)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="优化版点阵字体生成器")
    parser.add_argument("-ff", "--font-file", required=True)
    parser.add_argument("-fs", "--font-size", type=int, default=16)
    parser.add_argument("-tf", "--text-file")
    parser.add_argument("-t", "--text")
    parser.add_argument("-bfn", "--bitmap-font-name")
    args = parser.parse_args()
    
    
    bmf_file = run(args.font_file, args.font_size, 
                  text_file=args.text_file, 
                  text=args.text,
                  bitmap_fonts_name=args.bitmap_font_name)
    
    #bmf_file = "MiSans-Normal-36615-16.bmf"
    bmf = load_bmf(bmf_file)
    print("查找'中'字(0x4E2D):", bmf.get_char(0x4E2D))
    bmf.print_char("c", "a")