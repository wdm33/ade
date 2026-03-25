// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

/// Exact rational number using arbitrary-precision BigInt numerator and denominator.
///
/// Invariants:
/// - Denominator is always > 0
/// - Stored in reduced form (GCD = 1) after construction
///
/// No floating point arithmetic — all operations are exact integer math.
/// Uses BigInt to match Haskell's arbitrary-precision Integer, eliminating
/// overflow in intermediate computations (e.g., per-pool bracket formula).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    num: BigInt,
    den: BigInt,
}

impl Rational {
    /// Create a new rational number, reducing to canonical form.
    ///
    /// Returns None if denominator is zero.
    pub fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let mut r = Rational {
            num: BigInt::from(num),
            den: BigInt::from(den),
        };
        r.reduce();
        Some(r)
    }

    /// Create a rational from an integer (denominator = 1).
    pub fn from_integer(n: i128) -> Self {
        Rational {
            num: BigInt::from(n),
            den: BigInt::one(),
        }
    }

    /// Zero.
    pub fn zero() -> Self {
        Rational {
            num: BigInt::zero(),
            den: BigInt::one(),
        }
    }

    /// One.
    pub fn one() -> Self {
        Rational {
            num: BigInt::one(),
            den: BigInt::one(),
        }
    }

    /// Numerator (as i128 — truncates if value exceeds i128 range).
    ///
    /// For protocol parameter rationals and final floor results, this always fits.
    /// For intermediate computations, use the BigInt methods directly.
    pub fn numerator(&self) -> i128 {
        bigint_to_i128(&self.num)
    }

    /// Denominator (always > 0, as i128 — truncates if value exceeds i128 range).
    pub fn denominator(&self) -> i128 {
        bigint_to_i128(&self.den)
    }

    /// Floor: largest integer <= self.
    ///
    /// For positive values: num / den (truncating division).
    /// For negative values: rounds toward negative infinity.
    pub fn floor(&self) -> i128 {
        let (q, r) = self.num.div_rem(&self.den);
        if r.is_negative() {
            bigint_to_i128(&(q - BigInt::one()))
        } else {
            bigint_to_i128(&q)
        }
    }

    /// Ceiling: smallest integer >= self.
    pub fn ceiling(&self) -> i128 {
        let (q, r) = self.num.div_rem(&self.den);
        if r.is_positive() {
            bigint_to_i128(&(q + BigInt::one()))
        } else {
            bigint_to_i128(&q)
        }
    }

    /// Checked addition. Always succeeds with BigInt (no overflow).
    pub fn checked_add(&self, other: &Rational) -> Option<Rational> {
        let num = &self.num * &other.den + &other.num * &self.den;
        let den = &self.den * &other.den;
        let mut r = Rational { num, den };
        r.reduce();
        Some(r)
    }

    /// Checked subtraction. Always succeeds with BigInt (no overflow).
    pub fn checked_sub(&self, other: &Rational) -> Option<Rational> {
        let num = &self.num * &other.den - &other.num * &self.den;
        let den = &self.den * &other.den;
        let mut r = Rational { num, den };
        r.reduce();
        Some(r)
    }

    /// Checked multiplication. Always succeeds with BigInt (no overflow).
    pub fn checked_mul(&self, other: &Rational) -> Option<Rational> {
        let num = &self.num * &other.num;
        let den = &self.den * &other.den;
        let mut r = Rational { num, den };
        r.reduce();
        Some(r)
    }

    /// Checked division. Returns None only if dividing by zero.
    pub fn checked_div(&self, other: &Rational) -> Option<Rational> {
        if other.num.is_zero() {
            return None;
        }
        let num = &self.num * &other.den;
        let den = &self.den * &other.num;
        let mut r = Rational { num, den };
        r.reduce();
        Some(r)
    }

    /// Returns true if this rational is non-negative.
    pub fn is_non_negative(&self) -> bool {
        !self.num.is_negative()
    }

    /// Reduce to canonical form: GCD = 1, denominator > 0.
    fn reduce(&mut self) {
        if self.num.is_zero() {
            self.den = BigInt::one();
            return;
        }

        // Ensure denominator is positive
        if self.den.is_negative() {
            self.num = -&self.num;
            self.den = -&self.den;
        }

        let g = self.num.abs().gcd(&self.den);
        if g > BigInt::one() {
            self.num /= &g;
            self.den /= &g;
        }
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // a/b vs c/d => a*d vs c*b (both denominators are positive)
        let lhs = &self.num * &other.den;
        let rhs = &other.num * &self.den;
        lhs.cmp(&rhs)
    }
}

impl core::fmt::Display for Rational {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.den == BigInt::one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// Convert BigInt to i128, clamping to i128::MIN/MAX on overflow.
fn bigint_to_i128(b: &BigInt) -> i128 {
    use num_traits::ToPrimitive;
    b.to_i128().unwrap_or_else(|| {
        if b.is_negative() {
            i128::MIN
        } else {
            i128::MAX
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_reduces_to_canonical_form() {
        let r = Rational::new(4, 6).unwrap();
        assert_eq!(r.numerator(), 2);
        assert_eq!(r.denominator(), 3);
    }

    #[test]
    fn new_normalizes_negative_denominator() {
        let r = Rational::new(3, -5).unwrap();
        assert_eq!(r.numerator(), -3);
        assert_eq!(r.denominator(), 5);
    }

    #[test]
    fn new_zero_denominator_returns_none() {
        assert!(Rational::new(1, 0).is_none());
    }

    #[test]
    fn zero_rational() {
        let r = Rational::new(0, 42).unwrap();
        assert_eq!(r.numerator(), 0);
        assert_eq!(r.denominator(), 1);
    }

    #[test]
    fn from_integer() {
        let r = Rational::from_integer(42);
        assert_eq!(r.numerator(), 42);
        assert_eq!(r.denominator(), 1);
    }

    // -----------------------------------------------------------------------
    // Addition
    // -----------------------------------------------------------------------

    #[test]
    fn add_simple() {
        let a = Rational::new(1, 3).unwrap();
        let b = Rational::new(1, 6).unwrap();
        let c = a.checked_add(&b).unwrap();
        assert_eq!(c, Rational::new(1, 2).unwrap());
    }

    #[test]
    fn add_integers() {
        let a = Rational::from_integer(3);
        let b = Rational::from_integer(4);
        let c = a.checked_add(&b).unwrap();
        assert_eq!(c, Rational::from_integer(7));
    }

    #[test]
    fn add_negative() {
        let a = Rational::new(1, 2).unwrap();
        let b = Rational::new(-1, 3).unwrap();
        let c = a.checked_add(&b).unwrap();
        assert_eq!(c, Rational::new(1, 6).unwrap());
    }

    // -----------------------------------------------------------------------
    // Subtraction
    // -----------------------------------------------------------------------

    #[test]
    fn sub_simple() {
        let a = Rational::new(1, 2).unwrap();
        let b = Rational::new(1, 3).unwrap();
        let c = a.checked_sub(&b).unwrap();
        assert_eq!(c, Rational::new(1, 6).unwrap());
    }

    #[test]
    fn sub_to_negative() {
        let a = Rational::new(1, 4).unwrap();
        let b = Rational::new(1, 2).unwrap();
        let c = a.checked_sub(&b).unwrap();
        assert_eq!(c, Rational::new(-1, 4).unwrap());
    }

    // -----------------------------------------------------------------------
    // Multiplication
    // -----------------------------------------------------------------------

    #[test]
    fn mul_simple() {
        let a = Rational::new(2, 3).unwrap();
        let b = Rational::new(3, 4).unwrap();
        let c = a.checked_mul(&b).unwrap();
        assert_eq!(c, Rational::new(1, 2).unwrap());
    }

    #[test]
    fn mul_by_zero() {
        let a = Rational::new(5, 7).unwrap();
        let b = Rational::zero();
        let c = a.checked_mul(&b).unwrap();
        assert_eq!(c, Rational::zero());
    }

    #[test]
    fn mul_negative() {
        let a = Rational::new(-2, 3).unwrap();
        let b = Rational::new(3, 5).unwrap();
        let c = a.checked_mul(&b).unwrap();
        assert_eq!(c, Rational::new(-2, 5).unwrap());
    }

    // -----------------------------------------------------------------------
    // Division
    // -----------------------------------------------------------------------

    #[test]
    fn div_simple() {
        let a = Rational::new(1, 2).unwrap();
        let b = Rational::new(3, 4).unwrap();
        let c = a.checked_div(&b).unwrap();
        assert_eq!(c, Rational::new(2, 3).unwrap());
    }

    #[test]
    fn div_by_zero_returns_none() {
        let a = Rational::new(1, 2).unwrap();
        let b = Rational::zero();
        assert!(a.checked_div(&b).is_none());
    }

    // -----------------------------------------------------------------------
    // Floor
    // -----------------------------------------------------------------------

    #[test]
    fn floor_positive_exact() {
        let r = Rational::new(6, 3).unwrap();
        assert_eq!(r.floor(), 2);
    }

    #[test]
    fn floor_positive_truncates() {
        let r = Rational::new(7, 3).unwrap();
        assert_eq!(r.floor(), 2);
    }

    #[test]
    fn floor_negative_rounds_down() {
        let r = Rational::new(-7, 2).unwrap();
        assert_eq!(r.floor(), -4);
    }

    #[test]
    fn floor_negative_exact() {
        let r = Rational::new(-6, 3).unwrap();
        assert_eq!(r.floor(), -2);
    }

    #[test]
    fn floor_zero() {
        let r = Rational::zero();
        assert_eq!(r.floor(), 0);
    }

    // -----------------------------------------------------------------------
    // Ceiling
    // -----------------------------------------------------------------------

    #[test]
    fn ceiling_positive_exact() {
        let r = Rational::new(6, 3).unwrap();
        assert_eq!(r.ceiling(), 2);
    }

    #[test]
    fn ceiling_positive_rounds_up() {
        let r = Rational::new(7, 3).unwrap();
        assert_eq!(r.ceiling(), 3);
    }

    #[test]
    fn ceiling_negative_truncates() {
        let r = Rational::new(-7, 2).unwrap();
        assert_eq!(r.ceiling(), -3);
    }

    // -----------------------------------------------------------------------
    // Ordering
    // -----------------------------------------------------------------------

    #[test]
    fn ordering() {
        let a = Rational::new(1, 3).unwrap();
        let b = Rational::new(1, 2).unwrap();
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ordering_equal() {
        let a = Rational::new(2, 4).unwrap();
        let b = Rational::new(1, 2).unwrap();
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------

    #[test]
    fn display_integer() {
        let r = Rational::from_integer(42);
        assert_eq!(format!("{r}"), "42");
    }

    #[test]
    fn display_fraction() {
        let r = Rational::new(1, 3).unwrap();
        assert_eq!(format!("{r}"), "1/3");
    }

    // -----------------------------------------------------------------------
    // Properties
    // -----------------------------------------------------------------------

    #[test]
    fn is_non_negative() {
        assert!(Rational::from_integer(0).is_non_negative());
        assert!(Rational::from_integer(1).is_non_negative());
        assert!(!Rational::from_integer(-1).is_non_negative());
    }

    #[test]
    fn determinism_add_sub_round_trip() {
        let a = Rational::new(7, 11).unwrap();
        let b = Rational::new(3, 13).unwrap();
        let sum = a.checked_add(&b).unwrap();
        let diff = sum.checked_sub(&b).unwrap();
        assert_eq!(diff, a);
    }

    #[test]
    fn determinism_mul_div_round_trip() {
        let a = Rational::new(7, 11).unwrap();
        let b = Rational::new(3, 13).unwrap();
        let product = a.checked_mul(&b).unwrap();
        let quotient = product.checked_div(&b).unwrap();
        assert_eq!(quotient, a);
    }

    #[test]
    fn one_is_multiplicative_identity() {
        let a = Rational::new(7, 13).unwrap();
        let result = a.checked_mul(&Rational::one()).unwrap();
        assert_eq!(result, a);
    }

    #[test]
    fn zero_is_additive_identity() {
        let a = Rational::new(7, 13).unwrap();
        let result = a.checked_add(&Rational::zero()).unwrap();
        assert_eq!(result, a);
    }

    // -----------------------------------------------------------------------
    // Canonical encoding (T-26A.3)
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_form_matches_oracle_encoding() {
        // Oracle encodes rationals as tag(30, [n, d]) in lowest terms.
        // Our Rational::new() must produce the same (n, d) as the oracle.

        // treasury_growth: oracle = [1, 5], our default = Rational::new(2, 10)
        let r = Rational::new(2, 10).unwrap();
        assert_eq!(r.numerator(), 1, "2/10 must reduce to 1/5");
        assert_eq!(r.denominator(), 5);

        // pool_influence: oracle = [3, 10]
        let r = Rational::new(3, 10).unwrap();
        assert_eq!(r.numerator(), 3);
        assert_eq!(r.denominator(), 10);

        // monetary_expansion: oracle = [3, 1000]
        let r = Rational::new(3, 1000).unwrap();
        assert_eq!(r.numerator(), 3);
        assert_eq!(r.denominator(), 1000);

        // decentralization: oracle = [8, 25] (Shelley ep236)
        let r = Rational::new(8, 25).unwrap();
        assert_eq!(r.numerator(), 8);
        assert_eq!(r.denominator(), 25);

        // Equivalent representations must reduce to same canonical form
        let a = Rational::new(2, 10).unwrap();
        let b = Rational::new(1, 5).unwrap();
        let c = Rational::new(4, 20).unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.numerator(), 1);
        assert_eq!(a.denominator(), 5);
    }

    #[test]
    fn already_canonical_unchanged() {
        let r = Rational::new(1, 3).unwrap();
        assert_eq!(r.numerator(), 1);
        assert_eq!(r.denominator(), 3);
    }

    // -----------------------------------------------------------------------
    // BigInt precision: values that would overflow i128
    // -----------------------------------------------------------------------

    #[test]
    fn large_intermediate_no_overflow() {
        // Simulate a computation similar to the bracket formula:
        // pool_reward_pot * bracket / (1 + a0) where pot is ~30T
        let pot = Rational::from_integer(30_000_000_000_000); // 30T lovelace
        let bracket = Rational::new(1, 500).unwrap(); // sigma' ≈ 1/500
        let one_plus_a0 = Rational::new(13, 10).unwrap(); // 1 + 3/10

        // With i128, pot * bracket could overflow if bracket had large num/den
        let result = pot.checked_mul(&bracket)
            .and_then(|r| r.checked_div(&one_plus_a0))
            .unwrap();
        assert!(result.floor() > 0);
    }

    #[test]
    fn chain_operations_exact() {
        // Chain of operations that would accumulate precision loss with i128
        let total_stake = 20_400_000_000_000_000i128; // 20.4B ADA in lovelace
        let pool_stake = 50_000_000_000_000i128; // 50M ADA
        let sigma = Rational::new(pool_stake, total_stake).unwrap();
        let z = Rational::new(1, 500).unwrap();
        let a0 = Rational::new(3, 10).unwrap();
        let s = Rational::new(1_000_000_000_000, total_stake).unwrap(); // 1M ADA pledge

        // bracket = σ' + s' * a0 * (σ' - s' * (z - σ') / z)
        let z_minus_sigma = z.checked_sub(&sigma).unwrap();
        let inner = s.checked_mul(&z_minus_sigma).unwrap();
        let inner_div_z = inner.checked_div(&z).unwrap();
        let sigma_minus = sigma.checked_sub(&inner_div_z).unwrap();
        let pledge_term = s.checked_mul(&a0).unwrap()
            .checked_mul(&sigma_minus).unwrap();
        let bracket = sigma.checked_add(&pledge_term).unwrap();

        // Should be a small positive rational
        assert!(bracket.floor() >= 0);
        assert!(bracket.numerator() > 0);
    }
}
