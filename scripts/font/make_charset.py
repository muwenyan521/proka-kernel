import unicodedata

def generate_enhanced_unicode(output_file="enhanced_unicode.txt"):
    """生成增强版Unicode字符集，包含多语言支持和专业符号"""
    
    # 包含的Unicode范围（按类别分组）
    included_ranges = [
        # === 拉丁语系 ===
        (0x0000, 0x007F),    # 基本拉丁字母
        (0x0080, 0x00FF),    # 拉丁-1补充
        (0x0100, 0x017F),    # 拉丁扩展-A
        (0x0180, 0x024F),    # 拉丁扩展-B
        
        # === 西里尔字母 ===
        #(0x0400, 0x04FF),    # 西里尔字母基础;
        #(0x0500, 0x052F),    # 西里尔字母补充;
        
        # === 阿拉伯语系 ===
        #(0x0600, 0x06FF),    # 阿拉伯文基础;
        #(0x0750, 0x077F),    # 阿拉伯文扩展;
        #(0x08A0, 0x08FF),    # 阿拉伯文扩展-A;
        
        # === 印度系文字 ===
        #(0x0900, 0x097F),    # 梵文字母;
        #(0x0980, 0x09FF),    # 孟加拉文;
        #(0x0A00, 0x0A7F),    # 古木基文;
        
        
        # === 东亚文字 ===
        (0x4E00, 0x9FFF),    # 中日韩统一表意文字
        #(0x3040, 0x309F),    # 平假名;
        #(0x30A0, 0x30FF),    # 片假名;
        #(0xAC00, 0xD7AF),    # 韩文音节;
        #(0x3100, 0x312F),    # 注音符号;
        #(0x3190, 0x319F),    # 汉字训读标记;
        
        # === 符号系统 ===
        (0x2000, 0x206F),    # 通用标点
        (0x20A0, 0x20CF),    # 货币符号
        #(0x2100, 0x214F),    # 类字母符号;
        #(0x2150, 0x218F),    # 数字形式;
        (0x2190, 0x21FF),    # 箭头
        #(0x2200, 0x22FF),    # 数学运算符;
        (0x2300, 0x23FF),    # 技术符号
        (0x2500, 0x257F),    # 制表符
        (0x2580, 0x259F),    # 方块元素
        (0x25A0, 0x25FF),    # 几何图形
        #(0x2600, 0x26FF),    # 杂项符号;
        #(0x2700, 0x27BF),    # 装饰符号;
        #(0x27C0, 0x27EF),    # 数学箭头扩展;
        #(0x2900, 0x297F),    # 补充箭头-B;
        #(0x2A00, 0x2AFF),    # 数学运算符扩展;
        
        # === 中日韩符号 ===
        (0x3000, 0x303F),    # CJK符号和标点
        #(0xFE10, 0xFE1F),    # 竖排形式;
        (0xFF00, 0xFFEF),    # 半角/全角形式
        #(0xF900, 0xFAFF),    # CJK兼容汉字;
        
        
    ]

    # 排除范围
    excluded_ranges = [
        (0x0000, 0x001F),    # ASCII控制字符
        (0x007F, 0x009F),    # C1控制字符
        (0xD800, 0xDFFF),    # 代理对区域
        (0xFDD0, 0xFDEF),    # 非字符区域
        (0xE000, 0xF8FF),    # 私有使用区
        (0x1D12, 0x1D12),    # 易混淆数字符号
    ]

    # 易混淆字符黑名单
    confusing_chars = {
    }

    with open(output_file, "w", encoding="utf-8", errors="replace") as f:
        # 使用生成器优化内存
        def char_generator():
            for start, end in included_ranges:
                
                for cp in range(start, end + 1):
                    # 基础排除检查
                    if any(low <= cp <= high for (low, high) in excluded_ranges):
                        continue
                    if cp in confusing_chars:
                        continue
                    if (cp & 0xFFFE) == 0xFFFE:  # 非字符码点
                        continue

                    try:
                        char = chr(cp)
                        # 使用unicodedata过滤未分配字符
                        if unicodedata.category(char) == 'Cn':
                            continue
                        # 过滤组合字符和不可见字符
                        if unicodedata.combining(char) > 0:
                            continue
                        if unicodedata.bidirectional(char) in ('BN', 'B'):
                            continue
                            
                        yield char
                    except Exception as e:
                        print(f"跳过无效码点 U+{cp:04X}: {str(e)}")

        # 分批写入文件
        buffer = []
        for char in char_generator():
            buffer.append(char)
            if len(buffer) >= 10000:  # 每1万个字符写入一次
                f.write(''.join(buffer))
                buffer = []
        if buffer:  # 写入剩余内容
            f.write(''.join(buffer))

if __name__ == "__main__":
    generate_enhanced_unicode()
    print("增强版Unicode字符集已生成到 enhanced_unicode.txt")