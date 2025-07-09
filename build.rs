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

    #[cfg(not(any(feature = "r1", feature = "r3", feature = "r4", feature = "r5")))]
    compile_error!("Please enable a chip revision feature that corresponds to the one you are using!");

    #[cfg(any(
        all(feature = "r1", feature = "r3"),
        all(feature = "r1", feature = "r4"),
        all(feature = "r1", feature = "r5"),
        all(feature = "r3", feature = "r4"),
        all(feature = "r3", feature = "r5"),
        all(feature = "r4", feature = "r5"),
    ))]
    compile_error!("Please enable only one chip revision that corresponds to the one you are using!");
}
