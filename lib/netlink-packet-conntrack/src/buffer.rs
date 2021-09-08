use crate::{NetfilterMessage, DecodeError};
use crate::utils::ParseableParametrized;

const BUF_MIN_LEN: usize = 2;

pub struct NetfilterBuffer<T> {
    buffer: T,
}

impl <T: AsRef<[u8]>> NetfilterBuffer<T> {
    pub fn new(buffer: T) -> NetfilterBuffer<T> {
        NetfilterBuffer {
            buffer
        }
    }

    pub fn length(&self) -> usize {
        self.buffer.as_ref().len()
    }

    pub fn new_checked(buffer: T) -> Result<Self, DecodeError> {
        let packet = Self::new(buffer);
        packet.check_len()?;
        Ok(packet)
    }

    pub(crate) fn check_len(&self) -> Result<(), DecodeError> {
        let len = self.buffer.as_ref().len();
        if len < BUF_MIN_LEN {
            return Err(format!(
                "invalid buffer: length is {} but packets are at least {} bytes",
                len, BUF_MIN_LEN
            ).into());
        }

        Ok(())
    }

    pub(crate) fn family(&self) -> u8 {
        self.buffer.as_ref()[0]
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> NetfilterBuffer<&'a T> {
    pub fn inner(&self) -> &'a [u8] {
        &self.buffer.as_ref()[..]
    }
}

impl<'a, T: AsRef<[u8]>> ParseableParametrized<NetfilterBuffer<&'a T>, u16> for NetfilterMessage {
    fn parse_with_param(buf: &NetfilterBuffer<&'a T>, message_type: u16) -> Result<Self, DecodeError> {
        use self::NetfilterMessage::*;

        buf.check_len()?;

        let message = match (message_type, buf.family()) {
            (0, AF_INET) => {
                let err = "invalid AF_INET response";
                let buf = inet
            }

            _ => return Err(format!("unknown message type {}", message_type).into())
        };

        Ok(message)
    }
}