//! Country rule.

use super::ClientFieldError;

/// ISO 3166-1 alpha-2 country codes.
const ISO_ALPHA2_CODES: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

/// A country in uppercase ISO 3166-1 alpha-2 form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Country(String);

impl Country {
    /// Trims and uppercases the text, then checks it against the ISO list.
    pub fn parse(raw: &str) -> Result<Self, ClientFieldError> {
        let code = raw.trim().to_uppercase();

        if !ISO_ALPHA2_CODES.contains(&code.as_str()) {
            return Err(ClientFieldError::UnknownCountry);
        }

        Ok(Self(code))
    }

    /// The uppercase code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ClientFieldError;

    #[test]
    fn country_is_trimmed_and_uppercased() {
        assert_eq!(Country::parse(" ar ").unwrap().as_str(), "AR");
    }

    #[test]
    fn country_accepts_known_iso_alpha2_code() {
        assert!(Country::parse("US").is_ok());
    }

    #[test]
    fn country_rejects_unknown_code() {
        assert_eq!(
            Country::parse("ZZ").unwrap_err(),
            ClientFieldError::UnknownCountry
        );
        assert_eq!(
            Country::parse("ARG").unwrap_err(),
            ClientFieldError::UnknownCountry
        );
    }
}
