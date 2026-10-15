
// `Float` to and from text as nova-runtime gives it, which is Rust's
// (spec docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md
// §6). Ties are decided exactly, on the double's bits.

// |v| exactly, as [m, e]: m × 2^e, m a BigInt.
function exactly(v) {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, v);
  const bits = view.getBigUint64(0);
  const biased = Number((bits >> 52n) & 0x7ffn);
  const fraction = bits & 0xfffffffffffffn;
  return biased === 0 ? [fraction, -1074] : [fraction | 0x10000000000000n, biased - 1075];
}

// The digits and exponent of JS's shortest text for a positive double: D ×
// 10^q, D a BigInt with no trailing zero.
function shortest(a) {
  const [mantissa, exp] = String(a).split("e");
  const [whole, fraction = ""] = mantissa.split(".");
  let digits = (whole + fraction).replace(/^0+/, "");
  let q = (exp === undefined ? 0 : Number(exp)) - fraction.length;
  while (digits.length > 1 && digits.endsWith("0")) {
    digits = digits.slice(0, -1);
    q += 1;
  }
  return [BigInt(digits), q];
}

// D × 10^q written out, with no exponent.
function plain(d, q) {
  const digits = d.toString();
  if (q >= 0) return digits + "0".repeat(q);
  const point = digits.length + q;
  return point > 0
    ? `${digits.slice(0, point)}.${digits.slice(point)}`
    : `0.${"0".repeat(-point)}${digits}`;
}

// Rust's `Display` for f64: the shortest digits that read back as `v`,
// never an exponent. Where two shortest texts are equally near, ECMAScript
// takes the even one and Rust the larger: so when JS's last digit is even
// and |v| lies exactly halfway to the next, take the next.
function floatText(v) {
  if (Number.isNaN(v)) return "NaN";
  if (v === Infinity) return "inf";
  if (v === -Infinity) return "-inf";
  if (v === 0) return Object.is(v, -0) ? "-0" : "0";
  const a = Math.abs(v);
  let [d, q] = shortest(a);
  if (d % 2n === 0n) {
    const [m, e] = exactly(a);
    let twice = 2n * m;
    let halfway = 2n * d + 1n;
    if (e >= 0) twice <<= BigInt(e);
    else halfway <<= BigInt(-e);
    if (q >= 0) halfway *= 10n ** BigInt(q);
    else twice *= 10n ** BigInt(-q);
    if (twice === halfway && Number(`${d + 1n}e${q}`) === a) d += 1n;
  }
  return (v < 0 ? "-" : "") + plain(d, q);
}

// Rust's `{:.N}` for f64, `places` clamped to 0..=17: the exact value
// rounded half to even. The sign stays, on -0.0 too.
function floatFixed(v, places) {
  if (Number.isNaN(v)) return "NaN";
  if (v === Infinity) return "inf";
  if (v === -Infinity) return "-inf";
  const p = Math.min(Math.max(Number(places), 0), 17);
  const [m, e] = exactly(Math.abs(v));
  let n;
  if (e >= 0) {
    n = (m << BigInt(e)) * 10n ** BigInt(p);
  } else {
    const num = m * 10n ** BigInt(p);
    const den = 1n << BigInt(-e);
    n = num / den;
    const twice = 2n * (num % den);
    if (twice > den || (twice === den && n % 2n === 1n)) n += 1n;
  }
  const digits = n.toString().padStart(p + 1, "0");
  const body = p > 0 ? `${digits.slice(0, -p)}.${digits.slice(-p)}` : digits;
  return (v < 0 || Object.is(v, -0) ? "-" : "") + body;
}

// Rust's `str::parse::<f64>`, or 0.0 where it fails (nova_rt_str_to_float's
// `unwrap_or(0.0)`): its grammar exactly, then `Number`, which rounds
// correctly as Rust does.
const FLOAT_TEXT = /^[+-]?(?:inf|infinity|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)$/i;
function floatFrom(s) {
  if (!FLOAT_TEXT.test(s)) return 0;
  const body = s.replace(/^[+-]/, "").toLowerCase();
  if (body === "inf" || body === "infinity") return s.startsWith("-") ? -Infinity : Infinity;
  if (body === "nan") return NaN;
  return Number(s);
}
