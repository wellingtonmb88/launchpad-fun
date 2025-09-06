use crate::{K, TOKEN_TOTAL_SUPPLY};

pub fn initial_virtual_asset_reserve(asset_rate: u64) -> u128 {
    let k = (K * 10000) / asset_rate;
    let a = (k as u128) * 10000 * (1e9 as u128);
    let b = a / TOKEN_TOTAL_SUPPLY;
    (b * (1e9 as u128)) / 10000 as u128
}

pub fn calc_token_amount_out(
    asset_amount_in: u64,
    current_k: u128,
    virtual_asset_reserve: u128,
    virtual_token_reserve: u128,
) -> u64 {
    let asset_amount_in = asset_amount_in as u128;
    let token_y_amount =
        virtual_token_reserve - (current_k / (virtual_asset_reserve + asset_amount_in));
    token_y_amount as u64
}

pub fn calc_asset_amount_out(
    token_amount_in: u64,
    current_k: u128,
    virtual_token_reserve: u128,
    virtual_asset_reserve: u128,
) -> u64 {
    let token_amount_in = token_amount_in as u128;
    let asset_amount_out =
        virtual_asset_reserve - (current_k / (virtual_token_reserve + token_amount_in));
    asset_amount_out as u64
}

#[cfg(test)]
mod test {
    use crate::TOKEN_TOTAL_SUPPLY;

    use super::*;

    #[test]
    fn test_initial_virtual_asset_reserve() {
        // 4.285.714,2857
        assert_eq!(initial_virtual_asset_reserve(7), 4_285_714_285_700_000);
    }

    #[test]
    fn test_calc_token_amount_out() {
        let current_asset_supply = initial_virtual_asset_reserve(7);
        let current_k = current_asset_supply * TOKEN_TOTAL_SUPPLY;
        assert_eq!(current_k, 4285714285700000000000000000000000);
        let result = calc_token_amount_out(
            990_000_000,
            current_k,
            current_asset_supply,
            TOKEN_TOTAL_SUPPLY,
        );
        assert_eq!(result, 230999946640); // 230,99994664
    }

    #[test]
    fn test_calc_asset_amount_out() {
        let current_asset_supply = initial_virtual_asset_reserve(7);
        let current_k = current_asset_supply * TOKEN_TOTAL_SUPPLY;
        assert_eq!(current_k, 4285714285700000000000000000000000);
        let result = calc_asset_amount_out(
            990_000_000,
            current_k,
            TOKEN_TOTAL_SUPPLY,
            current_asset_supply,
        );
        assert_eq!(result, 4242858); // 0,004242858
    }
}
