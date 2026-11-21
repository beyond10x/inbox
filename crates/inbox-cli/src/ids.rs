//! The ids a client draws: a ULID per message and per reply.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::failure::Failure;

/// The 32 symbols of Crockford's base32, in ULID order.
const SYMBOLS: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A fresh ULID: 48 bits of the milliseconds since 1970, then 80 bits from the OS random source,
/// as 26 symbols.
///
/// # Errors
///
/// [`Failure::Local`] when the OS random source does not answer.
pub fn ulid() -> Result<String, Failure> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let mut random = [0_u8; 10];
    getrandom::fill(&mut random)
        .map_err(|error| Failure::Local(format!("the OS random source failed: {error}")))?;
    Ok(encode(millis, random))
}

fn encode(millis: u128, random: [u8; 10]) -> String {
    let mut value = (millis & 0xFFFF_FFFF_FFFF) << 80;
    for (at, byte) in random.iter().enumerate() {
        value |= u128::from(*byte) << (8 * (9 - at));
    }
    (0..26)
        .map(|at| {
            let index = (value >> (5 * (25 - at))) & 31;
            char::from(SYMBOLS[usize::try_from(index).unwrap_or_default()])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use inbox::messaging::MessageId;

    use super::{encode, ulid};

    #[test]
    fn a_drawn_id_is_a_message_id_the_model_admits() {
        for _ in 0..100 {
            let id = ulid().unwrap();
            assert_eq!(MessageId(id.clone()).broken_constraint(), None, "{id}");
        }
    }

    #[test]
    fn the_time_comes_first_so_later_ids_sort_after_earlier_ones() {
        assert_eq!(encode(0, [0; 10]), "00000000000000000000000000");
        assert_eq!(encode(1, [0; 10]), "00000000010000000000000000");
        assert_eq!(encode(0, [255; 10]), "0000000000ZZZZZZZZZZZZZZZZ");
        assert!(encode(2, [0; 10]) > encode(1, [255; 10]));
    }
}
