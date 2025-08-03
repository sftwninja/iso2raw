use crate::edc_ecc;
use anyhow::{bail, Result};

pub const ISO_SECTOR_SIZE: usize = 2048;
pub const RAW_SECTOR_SIZE: usize = 2352;

pub const SYNC_PATTERN: [u8; 12] = [
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
];

#[derive(Debug, Clone, Copy)]
pub struct SectorAddress {
    pub minute: u8,
    pub second: u8,
    pub frame: u8,
}

impl SectorAddress {
    pub fn from_lba(lba: u32) -> Self {
        let lba_offset = lba + 150; // CD-ROM addresses start at 2 seconds (150 frames)

        let frame = (lba_offset % 75) as u8;
        let second = ((lba_offset / 75) % 60) as u8;
        let minute = ((lba_offset / 75) / 60) as u8;

        Self {
            minute,
            second,
            frame,
        }
    }

    pub fn to_bcd(&self) -> [u8; 3] {
        [
            Self::to_bcd_byte(self.minute),
            Self::to_bcd_byte(self.second),
            Self::to_bcd_byte(self.frame),
        ]
    }

    fn to_bcd_byte(value: u8) -> u8 {
        ((value / 10) << 4) | (value % 10)
    }
}

pub struct Mode1Sector {
    pub sync: [u8; 12],
    pub header: [u8; 4],
    pub user_data: [u8; 2048],
    pub edc: [u8; 4],
    pub zero: [u8; 8],
    pub ecc_p: [u8; 172],
    pub ecc_q: [u8; 104],
}

impl Mode1Sector {
    pub fn new(lba: u32, data: &[u8]) -> Result<Self> {
        if data.len() != ISO_SECTOR_SIZE {
            bail!(
                "Invalid ISO sector size: expected {}, got {}",
                ISO_SECTOR_SIZE,
                data.len()
            );
        }

        let address = SectorAddress::from_lba(lba);
        let bcd_address = address.to_bcd();

        let mut sector = Self {
            sync: SYNC_PATTERN,
            header: [bcd_address[0], bcd_address[1], bcd_address[2], 0x01], // Mode 1
            user_data: [0; 2048],
            edc: [0; 4],
            zero: [0; 8],
            ecc_p: [0; 172],
            ecc_q: [0; 104],
        };

        sector.user_data.copy_from_slice(data);

        Ok(sector)
    }

    pub fn calculate_edc_ecc(&mut self) {
        // Prepare complete sector for EDC/ECC calculation
        let mut sector = vec![0u8; RAW_SECTOR_SIZE];
        self.to_bytes(&mut sector);

        // Use vendored EDCRE implementation (thanks @github/alex-free!)
        edc_ecc::calc_mode1_edc(&mut sector);
        edc_ecc::calc_p_parity(&mut sector);
        edc_ecc::calc_q_parity(&mut sector);

        // Extract calculated values back to struct
        self.edc.copy_from_slice(&sector[2064..2068]);
        self.ecc_p.copy_from_slice(&sector[2076..2248]);
        self.ecc_q.copy_from_slice(&sector[2248..2352]);
    }

    pub fn to_bytes(&self, buffer: &mut [u8]) {
        if buffer.len() < RAW_SECTOR_SIZE {
            return;
        }

        let mut offset = 0;

        buffer[offset..offset + 12].copy_from_slice(&self.sync);
        offset += 12;

        buffer[offset..offset + 4].copy_from_slice(&self.header);
        offset += 4;

        buffer[offset..offset + 2048].copy_from_slice(&self.user_data);
        offset += 2048;

        buffer[offset..offset + 4].copy_from_slice(&self.edc);
        offset += 4;

        buffer[offset..offset + 8].copy_from_slice(&self.zero);
        offset += 8;

        buffer[offset..offset + 172].copy_from_slice(&self.ecc_p);
        offset += 172;

        buffer[offset..offset + 104].copy_from_slice(&self.ecc_q);
    }
}

pub fn convert_iso_to_raw(lba: u32, iso_data: &[u8]) -> Result<Vec<u8>> {
    let mut sector = Mode1Sector::new(lba, iso_data)?;
    sector.calculate_edc_ecc();

    let mut raw_data = vec![0u8; RAW_SECTOR_SIZE];
    sector.to_bytes(&mut raw_data);

    Ok(raw_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_address_conversion() {
        let addr = SectorAddress::from_lba(0);
        assert_eq!(addr.minute, 0);
        assert_eq!(addr.second, 2);
        assert_eq!(addr.frame, 0);

        let bcd = addr.to_bcd();
        assert_eq!(bcd, [0x00, 0x02, 0x00]);
    }

    #[test]
    fn test_mode1_sector_creation() {
        let data = vec![0u8; ISO_SECTOR_SIZE];
        let sector = Mode1Sector::new(0, &data).unwrap();

        assert_eq!(sector.sync, SYNC_PATTERN);
        assert_eq!(sector.header[3], 0x01); // Mode 1
    }
}
