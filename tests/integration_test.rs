use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_iso_to_raw_conversion() {
    // Create a small test ISO file
    let test_iso = "test.iso";
    let test_bin = "test.bin";
    
    // Create a test ISO with exactly 10 sectors (20480 bytes)
    let iso_data = vec![0xABu8; 2048 * 10];
    fs::write(test_iso, &iso_data).expect("Failed to create test ISO");
    
    // Run the converter
    let output = Command::new("cargo")
        .args(&["run", "--", test_iso, "-o", test_bin, "-q"])
        .output()
        .expect("Failed to execute iso2raw");
    
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("iso2raw failed");
    }
    
    // Check output file exists and has correct size
    assert!(Path::new(test_bin).exists());
    let bin_data = fs::read(test_bin).expect("Failed to read output file");
    assert_eq!(bin_data.len(), 2352 * 10, "Output file has incorrect size");
    
    // Verify basic structure of first sector
    // Check sync pattern
    assert_eq!(&bin_data[0..12], &[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    
    // Check mode byte
    assert_eq!(bin_data[15], 0x01); // Mode 1
    
    // Check user data
    assert_eq!(&bin_data[16..2064], &iso_data[0..2048]);
    
    // Clean up
    let _ = fs::remove_file(test_iso);
    let _ = fs::remove_file(test_bin);
}