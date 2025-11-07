#![allow(missing_docs)]
fn main() {
    #[cfg(feature = "r1")]
    println!("cargo:rebuild-if-changed=device_R1.yaml");
    #[cfg(feature = "r3")]
    println!("cargo:rebuild-if-changed=device_R3.yaml");
    #[cfg(feature = "r4")]
    println!("cargo:rebuild-if-changed=device_R4.yaml");
    #[cfg(feature = "r5")]
    println!("cargo:rebuild-if-changed=device_R5.yaml");
}
