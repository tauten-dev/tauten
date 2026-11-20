Job names may now also be written in double quotes, wherever a job name
is expected. Between the quotes any character is allowed except control
characters; `\"` stands for a quote and `\\` for a backslash, and no
other backslash sequence exists. After the escapes are replaced, the name
is 1 to 32 characters long. The closing quote must be followed by
whitespace or the end of the line. Quotes are only for job names.

A quoted name that breaks any rule above, including the length, is the
syntax error `bad quoted name`.

A quoted name and an unquoted name with the same text are the same job.

Names are printed without quotes, in the job table and in error messages.

Any script line longer than 4096 bytes, not counting its line ending, is
the syntax error `line too long`; this includes comment lines. The length
is checked before anything else on the line.

Add tests.
