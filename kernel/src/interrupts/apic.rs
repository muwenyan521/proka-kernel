use crate::libs::msr;
use crate::println;
use raw_cpuid::CpuId;
use x86_64::registers::model_specific::Msr;

pub fn apic_is_available() -> bool {
    let cpuid = CpuId::new();
    cpuid
        .get_feature_info()
        .map_or(false, |info| info.has_apic())
}

pub fn x2apic_is_available() -> bool {
    let cpuid = CpuId::new();
    cpuid
        .get_feature_info()
        .map_or(false, |info| info.has_x2apic())
}

pub fn enable_x2apic() {
    unsafe {
        let mut apic_base = Msr::new(msr::IA32_APIC_BASE);
        let mut apic_base_raw = apic_base.read();
        apic_base_raw |= 1 << 10;
        apic_base.write(apic_base_raw);
    }
}

pub fn init() {
    if apic_is_available() {
        if x2apic_is_available() {
            println!("x2APIC is available")
        } else {
            println!("APIC is available");
        }
    } else {
        println!("APIC is not available");
    }
}
