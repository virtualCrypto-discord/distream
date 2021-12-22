use crate::packet::{AudioDataFormat, construct_header, construct_packet, HEADER_SIZE};

/// # encode
/// encode audio data packet to DRAP.
pub fn encode(packet: &AudioDataFormat) -> Option<Vec<u8>> {
    match packet {
        AudioDataFormat::SpeakingStart(ss) =>
            construct_packet(1, &[ss.snowflake.to_be_bytes(), ss.timestamp.to_be_bytes()].concat()),
        AudioDataFormat::Audio(a) => {
            let sf = a.snowflake.to_be_bytes();
            let sq = a.sequence.to_be_bytes();
            let d = &a.data;
            let data_size = sf.len() + sq.len() + d.len();
            let h = construct_header(2, data_size)?;
            let mut packet: Vec<u8> = Vec::with_capacity(data_size + HEADER_SIZE);
            packet.extend_from_slice(&h);
            packet.extend_from_slice(&sf);
            packet.extend_from_slice(&sq);
            packet.extend_from_slice(&d);
            Some(packet)
        }
    }
}

