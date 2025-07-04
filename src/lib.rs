use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CompactSize {
    pub value: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BitcoinError {
    InsufficientBytes,
    InvalidFormat,
}

impl CompactSize {
    pub fn new(value: u64) -> Self {
        CompactSize { value }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if self.value <= 0xFC {
            vec![self.value as u8]
        } else if self.value <= 0xFFFF {
            let mut bytes = vec![0xFD];
            bytes.extend(&self.value.to_le_bytes()[..2]);
            bytes
        } else if self.value <= 0xFFFFFFFF {
            let mut bytes = vec![0xFE];
            bytes.extend(&self.value.to_le_bytes()[..4]);
            bytes
        } else {
            let mut bytes = vec![0xFF];
            bytes.extend(&self.value.to_le_bytes());
            bytes
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.is_empty() {
            return Err(BitcoinError::InsufficientBytes);
        }

        let (value, consumed) = match bytes[0] {
            0x00..=0xFC => (bytes[0] as u64, 1),
            0xFD => {
                if bytes.len() < 3 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u16::from_le_bytes([bytes[1], bytes[2]]) as u64;
                (value, 3)
            }
            0xFE => {
                if bytes.len() < 5 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                (value, 5)
            }
            0xFF => {
                if bytes.len() < 9 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]);
                (value, 9)
            }
        };

        Ok((CompactSize::new(value), consumed))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Txid(pub [u8; 32]);

impl Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_string = hex::encode(self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str: String = Deserialize::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid length for Txid"));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Txid(array))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        OutPoint {
            txid: Txid(txid),
            vout,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(36);
        bytes.extend(&self.txid.0);
        bytes.extend(&self.vout.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 36 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let txid = Txid(bytes[0..32].try_into().unwrap());
        let vout = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        Ok((OutPoint { txid, vout }, 36))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Script {
    pub bytes: Vec<u8>,
}

impl Script {
    pub fn new(bytes: Vec<u8>) -> Self {
        Script { bytes }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.bytes.clone();
        let length = CompactSize::new(bytes.len() as u64);
        let mut result = length.to_bytes();
        result.append(&mut bytes);
        result
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (length, consumed) = CompactSize::from_bytes(bytes)?;
        if bytes.len() < consumed + length.value as usize {
            return Err(BitcoinError::InsufficientBytes);
        }
        let script_bytes = bytes[consumed..(consumed + length.value as usize)].to_vec();
        Ok((Script::new(script_bytes), consumed + length.value as usize))
    }
}

impl Deref for Script {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Script,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn new(previous_output: OutPoint, script_sig: Script, sequence: u32) -> Self {
        TransactionInput {
            previous_output,
            script_sig,
            sequence,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.previous_output.to_bytes());
        bytes.extend(self.script_sig.to_bytes());
        bytes.extend(&self.sequence.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (previous_output, consumed) = OutPoint::from_bytes(bytes)?;
        let (script_sig, consumed_script) = Script::from_bytes(&bytes[consumed..])?;
        let sequence = u32::from_le_bytes(
            bytes[consumed + consumed_script..consumed + consumed_script + 4]
                .try_into()
                .unwrap(),
        );
        Ok((
            TransactionInput::new(previous_output, script_sig, sequence),
            consumed + consumed_script + 4,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub lock_time: u32,
}

impl BitcoinTransaction {
    pub fn new(version: u32, inputs: Vec<TransactionInput>, lock_time: u32) -> Self {
        BitcoinTransaction {
            version,
            inputs,
            lock_time,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.version.to_le_bytes());
        let input_count = CompactSize::new(self.inputs.len() as u64);
        bytes.extend(input_count.to_bytes());
        for input in &self.inputs {
            bytes.extend(input.to_bytes());
        }
        bytes.extend(&self.lock_time.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 8 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let (input_count, consumed) = CompactSize::from_bytes(&bytes[4..])?;
        let mut inputs = Vec::new();
        let mut total_consumed = consumed + 4;
        for _ in 0..input_count.value {
            let (input, consumed_input) = TransactionInput::from_bytes(&bytes[total_consumed..])?;
            inputs.push(input);
            total_consumed += consumed_input;
        }
        if bytes.len() < total_consumed + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let lock_time = u32::from_le_bytes(
            bytes[total_consumed..total_consumed + 4]
                .try_into()
                .unwrap(),
        );
        Ok((
            BitcoinTransaction::new(version, inputs, lock_time),
            total_consumed + 4,
        ))
    }
}

impl fmt::Display for BitcoinTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Version: {}\nLock Time: {}\nInputs:\n",
            self.version, self.lock_time
        )?;
        for input in &self.inputs {
            writeln!(
                f,
                "  Previous Output Vout: {}\n",
                input.previous_output.vout
            )?;
        }
        Ok(())
    }
}
