# kvconf specification

`kvconf` reads a configuration file and answers questions about it.

    kvconf [--env-file <path>] <command> ...

| Command | Prints on success |
|---|---|
| `get [--int\|--bool\|--str] <file> <key>` | the key's value: canonical form; with `--int`/`--bool` the plain number or `true`/`false`; with `--str` the string's text without quotes or escapes |
| `check <file>` | `ok: N keys` |
| `dump <file>` | one line `<key> = <canonical value>` per key, in key order |
| `set <file> <key> <value>` | the file's text with the key set (the file itself is not changed) |

Every printed line ends with `\n`.

## Files

A configuration file is UTF-8 text, read as lines (a line ends at `\n`; a
`\r` before it is ignored). Lines are numbered from 1. Leading and trailing
whitespace on a line is ignored. A line that is empty, or whose first
non-space character is `#`, is a comment.

Every other line is one of:

- `[<section>]`: the section name is a key. Until the next section line,
  every key defined in the file is prefixed with `<section>.`.
- `<key> = <value>`: defines the key (with the section prefix) to the value.
  Whitespace around `=` is ignored.

The **configuration root** is the directory of the file named on the command
line. Paths inside the program are relative to it, and errors name files by
that relative path; the file on the command line is named by its file name.

## Keys

A key is one or more segments separated by `.`. A segment is an ASCII
lower-case letter followed by any number of ASCII lower-case letters,
digits `0`–`9` and `_`. A whole key is at most 64 bytes, section prefix
included. Keys are case-sensitive. Key order (for `dump`) is byte order.

Each key is defined once. A key may not be a dotted prefix of another
defined key: `a` and `a.b` cannot both be defined.

## Values

| Value | Syntax | Canonical form |
|---|---|---|
| string | `"..."` with escapes `\"`, `\\`, `\n`, `\t`; any other character stands for itself | quoted, with `"`, `\`, newline and tab escaped |
| integer | see G1, G2 | decimal, `-` for negatives |
| boolean | `true` or `false` | `true` or `false` |
| list | `[` values separated by `,` `]`; elements are strings, integers or booleans; `[]` is empty | `[` elements joined by `, ` `]` |

Nothing may follow a complete value on its line.

## Layers

After the file is read, each defined key may be overridden from the
environment (G4): the variable `KVCONF_` + the key in upper case with every
`.` written as `__` (`server.port` → `KVCONF_SERVER__PORT`). Its text is
parsed as a value and replaces the file's value. Variables for keys that
the file does not define are ignored.

With `--env-file <path>` (a path relative to the current directory), the
environment is exactly the variables in that file, and the process
environment is not consulted at all. Each non-comment line of an env file is
`NAME=VALUE`; the value is the text after the first `=`, kept as it is.

## Errors

Processing stops at the first error. Nothing is printed on standard output;
standard error gets exactly one line starting `error: ` and the status is 1.

| Message | When |
|---|---|
| `error: <file>: line N: <reason>` | a line of a configuration file is invalid |
| `error: cannot read <file>: <detail>` | a configuration file cannot be read |
| `error: env <NAME>: <reason>` | an override variable is not a valid value |
| `error: cannot read env file: <detail>` | the env file cannot be read |
| `error: env file line N: expected NAME=VALUE` | a line of the env file is not `NAME=VALUE` |
| ``error: bad key `<text>` `` | a key given on the command line breaks the key grammar |
| ``error: no such key `<key>` `` | `get` of a key that is not defined |
| ``error: key `<key>` is not of type <type>`` | `get` with a type option (`int`, `bool`, `str`) the value does not have |
| ``error: bad value `<text>` `` | `set` with a value that is not a valid value |
| ``error: key conflict `<key>` `` | `set` of a new key that would conflict with a defined key |

Line reasons: `` expected `key = value` ``, ``bad key `<text>` ``,
``bad section `<text>` ``, ``bad value `<text>` ``, `value out of range`,
`unterminated string`, ``bad escape `\<c>` ``,
`unexpected text after the value`, ``duplicate key `<key>` ``,
``key conflict `<key>` ``.

A wrong command line prints a usage line on standard error with status 2.

## `set`

`set` checks the file as `check` does, then checks the key and the value.
If the key is defined by a line of the file, that line becomes
`<key as written on the line> = <value>`. Otherwise a line
`<key> = <value>` is added: at the end of the file if it has no section
lines, else just before the first section line. All other lines are kept
exactly; the value is written as given (trimmed). A new key that would
conflict with a defined key is a `key conflict` error.

## Ground rules

These apply to every feature, present and future.

- **G1. Integers.** Integers are signed 64-bit (from -9223372036854775808
  to 9223372036854775807). Every operation on them gives the exact result
  or the error `value out of range`. No operation fails because an
  intermediate value of a calculation is large: only the final result
  counts.
- **G2. Number syntax.** A number is an optional `-` followed by one or more
  ASCII digits `0`–`9`, and nothing else: no `+`, no spaces, no decimal
  point, no exponent, no other digit characters. Leading zeros are allowed.
  A rule may forbid the `-`.
- **G3. Text.** Files are UTF-8. String values may contain any Unicode.
  Keys are ASCII only, as the key grammar says. Lengths of text are counted
  in Unicode scalar values (Rust `char`s) unless a rule says bytes.
- **G4. Authority.** Configuration files are found only relative to the
  configuration root, never relative to the current directory, and a
  feature never reads a file outside the root. Variables come only from the
  environment the command line selects: the env file when `--env-file` is
  given, else the process environment.
- **G5. No crashes.** The program never crashes, panics or hangs, whatever
  the input. Every failure is reported as described under Errors.
- **G6. Output failures.** If standard output (or any other output the
  program writes) cannot be written, the program prints
  `error: cannot write output` on standard error and exits with status 1.
- **G7. Coverage.** Every count and every output format covers every
  state, event kind or value type, including later ones; new ones are
  listed after the existing ones, in the order the feature's
  description introduces them. Where a format has no notation for a
  value type, it shows such a value as a plain number of its smallest
  unit.
