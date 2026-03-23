// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Exact rational number using i128 numerator and denominator.
///
/// Invariants:
/// - Denominator is always > 0
/// - Stored in reduced form (GCD = 1) after construction
///
/// No floating point arithmetic — all operations are exact integer math.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    /// Create a new rational number, reducing to canonical form.
    ///
    /// Returns None if denominator is zero.
    pub fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let mut r = Rational { num, den };
        r.reduce();
        Some(r)
    }

    /// Create a rational from an integer (denominator = 1).
    pub fn from_integer(n: i128) -> Self {
        Rational { num: n, den: 1 }
    }

    /// Zero.
    pub fn zero() -> Self {
        Rational { num: 0, den: 1 }
    }

    /// One.
    pub fn one() -> Self {
        Rational { num: 1, den: 1 }
    }

    /// Numerator.
    pub fn numerator(&self) -> i128 {
        self.num
    }

    /// Denominator (always > 0).
    pub fn denominator(&self) -> i128 {
        self.den
    }

    /// Floor: largest integer <= self.
    ///
    /// For positive values: num / den (truncating division).
    /// For negative values: rounds toward negative infinity.
    pub fn floor(&self) -> i128 {
        if self.num >= 0 {
            self.num / self.den
        } else {
            // For negative: floor division
            // e.g., -7/2 = -4 (not -3)
            let abs_num = self.num.saturating_neg();
            let q = abs_num / self.den;
            let r = abs_num % self.den;
            if r == 0 {
                self.num / self.den
            } else {
                -(q + 1)
            }
        }
    }

    /// Ceiling: smallest integer >= self.
    pub fn ceiling(&self) -> i128 {
        if self.num >= 0 {
            let q = self.num / self.den;
            let r = self.num % self.den;
            if r == 0 { q } else { q + 1 }
        } else {
            // For negative: ceiling is just truncation
            self.num / self.den
        }
    }

    /// Checked addition.
    pub fn checked_add(&self, other: &Rational) -> Option<Rational> {
        // a/b + c/d = (a*d + c*b) / (b*d)
        let num = self.num.checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Rational::new(num, den)
    }

    /// Checked subtraction.
    pub fn checked_sub(&self, other: &Rational) -> Option<Rational> {
        // a/b - c/d = (a*d - c*b) / (b*d)
        let num = self.num.checked_mul(other.den)?
            .checked_sub(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Rational::new(num, den)
    }

    /// Checked multiplication.
    pub fn checked_mul(&self, other: &Rational) -> Option<Rational> {
        // a/b * c/d = (a*c) / (b*d)
        let num = self.num.checked_mul(other.num)?;
        let den = self.den.checked_mul(other.den)?;
        Rational::new(num, den)
    }

    /// Checked division.
    pub fn checked_div(&self, other: &Rational) -> Option<Rational> {
        if other.num == 0 {
            return None;
        }
        // a/b / c/d = (a*d) / (b*c)
        let num = self.num.checked_mul(other.den)?;
        let den = self.den.checked_mul(other.num)?;
        Rational::new(num, den)
    }

    /// Returns true if this rational is non-negative.
    pub fn is_non_negative(&self) -> bool {
        self.num >= 0
    }

    /// Reduce to canonical form: GCD = 1, denominator > 0.
    fn reduce(&mut self) {
        if self.num == 0 {
            self.den = 1;
            return;
        }

        // Ensure denominator is positive
        if self.den < 0 {
            self.num = self.num.saturating_neg();
            self.den = self.den.saturating_neg();
        }

        let g = gcd(abs_i128(self.num), abs_i128(self.den));
        if g > 1 {
            self.num /= g;
            self.den /= g;
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
        let lhs = self.num.saturating_mul(other.den);
        let rhs = other.num.saturating_mul(self.den);
        lhs.cmp(&rhs)
    }
}

impl core::fmt::Display for Rational {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// Compute GCD using Euclidean algorithm. Inputs must be non-negative.
fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Absolute value for i128, handling MIN correctly.
fn abs_i128(x: i128) -> i128 {
    if x < 0 {
        x.saturating_neg()
    } else {
        x
    }
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

    #[test]
    fn gcd_helper() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
    }
}
