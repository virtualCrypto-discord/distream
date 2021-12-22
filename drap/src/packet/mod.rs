pub mod decoder;
pub mod encoder;

use std::convert::TryInto;

// 2^13-1
const MAX_PACKET_DATA_SIZE: usize = 0x1FFF;

// size 13bit + type 3bit
const HEADER_SIZE: usize = 2;

const MAX_PACKET_SIZE: usize = HEADER_SIZE + MAX_PACKET_DATA_SIZE;

#[derive(Debug, PartialEq, Eq)]
pub struct Audio {
    pub snowflake: u64,
    pub sequence: u16,
    pub data: Vec<u8>,
}

impl Audio {
    fn new(data: &[u8]) -> Option<Self>
    {
        if data.len() < 10 {
            return None;
        }
        let (snowflake, data) = data.split_at(std::mem::size_of::<u64>());
        let (sequence, data) = data.split_at(std::mem::size_of::<u16>());

        Some(Audio { snowflake: u64::from_be_bytes(snowflake.try_into().unwrap()), sequence: u16::from_be_bytes(sequence.try_into().unwrap()), data: data.to_vec() })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SpeakingStart {
    pub snowflake: u64,
    pub timestamp: u64,
}

impl SpeakingStart {
    fn new(data: &[u8]) -> Option<Self>
    {
        if data.len() != 16 {
            return None;
        }
        let (snowflake, timestamp) = data.split_at(std::mem::size_of::<u64>());
        Some(SpeakingStart { snowflake: u64::from_be_bytes(snowflake.try_into().unwrap()), timestamp: u64::from_be_bytes(timestamp.try_into().unwrap()) })
    }
}


#[derive(Debug, PartialEq, Eq)]
enum PacketKind {
    SpeakingStart,
    Audio,
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecoderError<E> {
    UnexpectedTermination,
    IllegalSize,
    IllegalSizedObject,
    UnknownKind,
    IteratorError(E),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AudioDataFormat {
    SpeakingStart(SpeakingStart),
    Audio(Audio),
}

fn construct_header(ty: u8, data_size: usize) -> Option<[u8; 2]> {
    if data_size > MAX_PACKET_DATA_SIZE {
        return None;
    }
    let first = (data_size >> 5) as u8;
    let second_size = ((data_size & 0x1F) << 3) as u8;
    let second = second_size | ty;
    return Some([first, second]);
}

fn construct_packet(ty: u8, d: &[u8]) -> Option<Vec<u8>> {
    let data_size = d.len();
    let h = construct_header(ty, data_size)?;
    let mut packet: Vec<u8> = Vec::with_capacity(data_size + HEADER_SIZE);
    packet.extend_from_slice(&h);
    packet.extend_from_slice(d);
    Some(packet)
}


#[cfg(test)]
mod tests {
    use crate::packet::encoder::encode;
    use crate::PacketDecoder;
    use super::*;

    fn to_failable_iterator(vec: Vec<u8>) -> impl Iterator<Item=Result<u8, ()>> {
        vec.into_iter().map(Ok)
    }

    #[test]
    fn construct_binary_ss() {
        let expect = vec![0x00, (16u8 << 3) | 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        assert_eq!(construct_packet(1, &[0u8, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2]).unwrap(), expect)
    }

    #[test]
    fn construct_binary_a() {
        let expect = vec![0x00, (14u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6];
        assert_eq!(construct_packet(2, &[0u8, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6]).unwrap(), expect)
    }

    #[test]
    fn construct_binary_long() {
        let mut expect = vec![0xFF, (0xFF << 3) | 2];
        let data: Vec<u8> = [0u8].iter().cycle().map(|&e| e).take(MAX_PACKET_DATA_SIZE).collect();
        expect.extend(&data);
        assert_eq!(construct_packet(2, &data).unwrap(), expect)
    }

    #[test]
    fn construct_binary_too_longer() {
        let mut expect = vec![0xFF, (0xFF << 3) | 2];
        let data: Vec<u8> = [0u8].iter().cycle().map(|&e| e).take(MAX_PACKET_DATA_SIZE + 1).collect();
        expect.extend(&data);
        assert_eq!(construct_packet(2, &data), None)
    }

    #[test]
    fn encode_decode_ss() {
        let s = AudioDataFormat::SpeakingStart(SpeakingStart { snowflake: 1, timestamp: 2 });
        let bin = encode(&s).unwrap();
        let mut iter = to_failable_iterator(bin);
        let mut decoder = PacketDecoder::new(&mut iter);
        assert_eq!(decoder.next().unwrap().unwrap(), s);
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn encode_decode_a() {
        let s = AudioDataFormat::Audio(Audio { snowflake: 1, sequence: 2, data: vec![3, 4, 5, 6] });
        let bin = encode(&s).unwrap();
        let mut iter = to_failable_iterator(bin);
        let mut decoder = PacketDecoder::new(&mut iter);
        assert_eq!(decoder.next().unwrap().unwrap(), s);
        assert_eq!(decoder.next(), None);
    }
}
