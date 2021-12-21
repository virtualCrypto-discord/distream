use crate::packet::{Audio, AudioDataFormat, DecoderError, MAX_PACKET_SIZE, PacketKind, SpeakingStart};


fn decode_header<E>(itr: &mut dyn Iterator<Item=Result<u8, E>>) -> Option<Result<(usize, PacketKind), DecoderError<E>>> {
    let first = itr.next()?;
    Some(_decode_header(first, itr))
}

fn _decode_header<E>(first: Result<u8, E>, itr: &mut dyn Iterator<Item=Result<u8, E>>) -> Result<(usize, PacketKind), DecoderError<E>> {
    let first = first.map_err(|x| DecoderError::IteratorError(x))? as usize;
    let second = itr.next().ok_or(DecoderError::UnexpectedTermination)?.map_err(|x| DecoderError::IteratorError(x))? as usize;
    let len = (first) << 5 | ((second) >> 3);
    let ty = second & 0x07;
    Ok((len, match ty {
        1 => PacketKind::SpeakingStart,
        2 => PacketKind::Audio,
        _ => PacketKind::Unknown
    }))
}

fn advance_by<T>(itr: &mut dyn Iterator<Item=T>, n: usize) -> Result<(), usize> {
    for x in 0..n {
        itr.next().ok_or(x)?;
    }
    Ok(())
}


pub struct PacketDecoder<'a, E> {
    itr: &'a mut dyn Iterator<Item=Result<u8, E>>,
}

impl<E> PacketDecoder<'_, E> {
    fn _next(&mut self) -> Option<Result<AudioDataFormat, DecoderError<E>>> {
        Some(match decode_header(&mut self.itr)? {
            Ok((size, kind)) => self.construct_packet(size, kind),
            Err(e) => Err(e)
        })
    }
    fn construct_packet(&mut self, size: usize, kind: PacketKind) -> Result<AudioDataFormat, DecoderError<E>> {
        if size > MAX_PACKET_SIZE {
            let _ = advance_by(&mut self.itr, size);
            return Err(DecoderError::IllegalSize);
        }
        let mut data: Vec<u8> = Vec::with_capacity(size);
        for x in 0..size {
            let v = self.itr.next().ok_or(DecoderError::UnexpectedTermination)?;
            match v {
                Ok(v) => data.push(v),
                Err(v) => {
                    let _ = advance_by(&mut self.itr, size - x - 1);
                    return Err(DecoderError::IteratorError(v));
                }
            }
        }
        match kind {
            PacketKind::SpeakingStart => {
                let packet = SpeakingStart::new(&data).ok_or(DecoderError::IllegalSizedObject)?;
                Ok(AudioDataFormat::SpeakingStart(packet))
            }
            PacketKind::Audio => {
                let packet = Audio::new(&data).ok_or(DecoderError::IllegalSizedObject)?;
                Ok(AudioDataFormat::Audio(packet))
            }
            PacketKind::Unknown => Err(DecoderError::UnknownKind)
        }
    }
    pub fn new<'a>(itr: &'a mut dyn Iterator<Item=Result<u8, E>>) -> PacketDecoder<'a, E> {
        PacketDecoder { itr: itr }
    }
}

impl<E> Iterator for PacketDecoder<'_, E> {
    type Item = Result<AudioDataFormat, DecoderError<E>>;
    fn next(&mut self) -> Option<Self::Item> {
        self._next()
    }
}


#[cfg(test)]
mod test {
    use super::*;

    fn to_failable_iterator(vec: Vec<u8>) -> impl Iterator<Item=Result<u8, ()>> {
        vec.into_iter().map(Ok)
    }

    #[test]
    fn decode_speaking_start_packet_header() {
        let d = vec![0x00, (16u8 << 3) | 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let mut iter = to_failable_iterator(d);
        assert_eq!(decode_header(&mut iter).unwrap().unwrap(), (16usize, PacketKind::SpeakingStart))
    }

    #[test]
    fn decode_audio_packet_header() {
        let d = vec![0x00, (14u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2];
        let mut iter = to_failable_iterator(d);
        assert_eq!(decode_header(&mut iter).unwrap().unwrap(), (14, PacketKind::Audio));
    }

    #[test]
    fn decode_speaking_start_packet() {
        let d = vec![0x00, (16u8 << 3) | 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];
        let mut iter = to_failable_iterator(d);
        let mut decoder = PacketDecoder::new(&mut iter);

        assert_eq!(decoder.next().unwrap().unwrap(), AudioDataFormat::SpeakingStart(SpeakingStart { snowflake: 1, timestamp: 2 }));
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn decode_audio_packet() {
        let d = vec![0x00, (14u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6];
        let mut iter = to_failable_iterator(d);
        let mut decoder = PacketDecoder::new(&mut iter);

        assert_eq!(decoder.next().unwrap().unwrap(), AudioDataFormat::Audio(Audio { snowflake: 1, sequence: 2, data: vec![3, 4, 5, 6] }));
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn decode_audio_packet2() {
        let d = vec![0x00, (13u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6];
        let mut iter = to_failable_iterator(d);
        let mut decoder = PacketDecoder::new(&mut iter);

        assert_eq!(decoder.next().unwrap().unwrap(), AudioDataFormat::Audio(Audio { snowflake: 1, sequence: 2, data: vec![3, 4, 5] }));
        assert_eq!(decoder.next(), Some(Err(DecoderError::UnexpectedTermination)));
    }

    #[test]
    fn decode_overrun() {
        let d = vec![0x00, (15u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6];
        let mut iter = to_failable_iterator(d);
        let mut decoder = PacketDecoder::new(&mut iter);

        assert_eq!(decoder.next().unwrap(), Err(DecoderError::UnexpectedTermination));
        assert_eq!(decoder.next(), None);
    }

    #[test]
    fn decode_stream() {
        let d = vec![0x00, (16u8 << 3) | 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0x00, (14u8 << 3) | 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 4, 5, 6];
        let mut iter = to_failable_iterator(d);
        let mut decoder = PacketDecoder::new(&mut iter);

        assert_eq!(decoder.next().unwrap().unwrap(), AudioDataFormat::SpeakingStart(SpeakingStart { snowflake: 1, timestamp: 2 }));
        assert_eq!(decoder.next().unwrap().unwrap(), AudioDataFormat::Audio(Audio { snowflake: 1, sequence: 2, data: vec![3, 4, 5, 6] }));
        assert_eq!(decoder.next(), None);
    }
}
