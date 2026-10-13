import BN from "bn.js";
import { assert } from "chai";
import { Decimal } from "decimal.js";

const DECIMAL_FRACTIONAL_BYTES = 6;
const DECIMAL_DIVISOR = new Decimal(2).pow(8 * DECIMAL_FRACTIONAL_BYTES);

export function fixedPointDiv(
  numerator: BN | Decimal | string | number,
  denominator: BN | Decimal | string | number
): Decimal {
  const num = convertNumberToDecimal(numerator);
  const den = convertNumberToDecimal(denominator);

  return num.mul(DECIMAL_DIVISOR).div(den).floor().div(DECIMAL_DIVISOR);
}

export function convertDecimalBitsToDecimal(value: BN): Decimal {
  let decimalValue = new Decimal(value.toString());
  return decimalValue.div(DECIMAL_DIVISOR);
}

export function convertNumberToDecimal(
  value: BN | BigInt | number | string | Decimal
): Decimal {
  if (typeof value === "string") {
    return new Decimal(value);
  } else if (value instanceof Decimal) {
    return value;
  } else {
    return new Decimal(value.toString());
  }
}

export const assertApprox = (
  a: Decimal | BN | BigInt | number | string,
  b: Decimal | BN | BigInt | number | string,
  tolerance: number,
  errorMessage?: string
) => {
  const aDecimal = convertNumberToDecimal(a);
  const bDecimal = convertNumberToDecimal(b);

  const diff = aDecimal.minus(bDecimal).abs();
  const allowedDifference = new Decimal(tolerance);

  assert.isTrue(
    diff.lessThanOrEqualTo(allowedDifference),
    `${
      errorMessage ? `${errorMessage}. ` : ""
    } Values are not approximately equal. Difference: ${diff.toString()}, Allowed Tolerance: ${tolerance}`
  );
};
