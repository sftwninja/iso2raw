use iso2raw::edc_ecc::calc_edc;
use iso2raw::converter::{Mode1Sector, SYNC_PATTERN, RAW_SECTOR_SIZE};

#[test]
fn test_edc_calculation_properties() {
    // Test that all zeros produces EDC of 0
    let data = vec![0u8; 2064];
    let edc = calc_edc(&data);
    assert_eq!(edc, 0, "EDC of all zeros should be 0");
    
    // Test that different data produces different EDC
    let data1 = vec![0xAAu8; 2064];
    let data2 = vec![0x55u8; 2064];
    let edc1 = calc_edc(&data1);
    let edc2 = calc_edc(&data2);
    assert_ne!(edc1, edc2, "Different data should produce different EDC values");
    
    // Test that EDC is deterministic
    let test_data = vec![0x12, 0x34, 0x56, 0x78];
    let edc_first = calc_edc(&test_data);
    let edc_second = calc_edc(&test_data);
    assert_eq!(edc_first, edc_second, "EDC calculation should be deterministic");
}

#[test]
fn test_complete_sector_generation() {
    // Create a test sector with known data
    let test_data = vec![0xAAu8; 2048];
    let mut sector = Mode1Sector::new(0, &test_data).unwrap();
    sector.calculate_edc_ecc();
    
    // Convert to bytes
    let mut raw_sector = vec![0u8; RAW_SECTOR_SIZE];
    sector.to_bytes(&mut raw_sector);
    
    // Verify structure
    assert_eq!(&raw_sector[0..12], &SYNC_PATTERN);
    assert_eq!(raw_sector[15], 0x01); // Mode 1
    assert_eq!(&raw_sector[16..2064], &test_data[..]);
    
    // Print EDC value for debugging
    let edc_bytes = &raw_sector[2064..2068];
    let edc_value = u32::from_le_bytes([edc_bytes[0], edc_bytes[1], edc_bytes[2], edc_bytes[3]]);
    println!("Generated EDC: 0x{:08X}", edc_value);
    
    // Verify ECC bytes are not all zeros
    let ecc_p = &raw_sector[2076..2248];
    let ecc_q = &raw_sector[2248..2352];
    assert!(!ecc_p.iter().all(|&b| b == 0), "P parity should not be all zeros");
    assert!(!ecc_q.iter().all(|&b| b == 0), "Q parity should not be all zeros");
}