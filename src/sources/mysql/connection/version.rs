use std::cmp::Ordering;

use super::Error;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(u8)]
pub enum Flavor {
    #[default]
    MySQL = 1,
    MariaDB = 2,
}

#[derive(Clone, Debug)]
pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
    flavor: Flavor,
}

impl From<(u8, u8)> for Version {
    fn from((major, minor): (u8, u8)) -> Self {
        Self {
            major,
            minor,
            patch: 0,
            flavor: Flavor::MySQL,
        }
    }
}

impl From<(u8, u8, u8)> for Version {
    fn from((major, minor, patch): (u8, u8, u8)) -> Self {
        Self {
            major,
            minor,
            patch,
            flavor: Flavor::MySQL,
        }
    }
}

impl PartialEq<f32> for Version {
    fn eq(&self, other: &f32) -> bool {
        let mut ver = self.minor as f32;
        while ver > 1.0 {
            ver /= 10.0;
        }

        ver += self.major as f32;

        ver.eq(other)
    }
}

impl PartialOrd<f32> for Version {
    fn partial_cmp(&self, other: &f32) -> Option<Ordering> {
        let mut ver = self.minor as f32;
        while ver > 1.0 {
            ver /= 10.0;
        }

        ver += self.major as f32;

        ver.partial_cmp(other)
    }
}

impl PartialEq<(u8, u8, u8)> for Version {
    fn eq(&self, other: &(u8, u8, u8)) -> bool {
        (self.major, self.minor, self.patch).eq(other)
    }
}

impl PartialOrd<(u8, u8, u8)> for Version {
    fn partial_cmp(&self, other: &(u8, u8, u8)) -> Option<Ordering> {
        (self.major, self.minor, self.patch).partial_cmp(other)
    }
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut fields = value.split(|c: char| !c.is_ascii_digit());

        let field = fields
            .next()
            .ok_or(Error::Protocol(format!("invalid server version {value:?}")))?;
        let major = field
            .parse()
            .map_err(|_| Error::Protocol(format!("invalid server version {value:?}")))?;

        let field = fields
            .next()
            .ok_or(Error::Protocol(format!("invalid server version {value:?}")))?;
        let minor = field
            .parse()
            .map_err(|_| Error::Protocol(format!("invalid server version {value:?}")))?;

        let field = fields
            .next()
            .ok_or(Error::Protocol(format!("invalid server version {value:?}")))?;
        let patch = field
            .parse()
            .map_err(|_| Error::Protocol(format!("invalid server version {value:?}")))?;

        let flavor = if value.contains("mariadb") {
            Flavor::MariaDB
        } else {
            Flavor::MySQL
        };

        Ok(Self {
            major,
            minor,
            patch,
            flavor,
        })
    }
}

impl Version {
    #[inline]
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }
}

#[cfg(test)]
impl Version {
    pub fn set(&mut self, major: u8, minor: u8, patch: u8) {
        self.major = major;
        self.minor = minor;
        self.patch = patch;
    }

    pub fn set_flavor(&mut self, flavor: Flavor) {
        self.flavor = flavor;
    }
}
