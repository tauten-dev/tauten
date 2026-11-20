Add an `include` directive to configuration files.

A line `include "<path>"` (the path written as a string value, with the
usual escapes) reads another configuration file at that point. The path is
relative to the directory of the file that contains the line. The included
file starts with no section, and its keys are defined as if they appeared
at the include line, under the usual rules: a key defined in two files is a
duplicate. After the include line, the including file's section continues.
Errors inside an included file name that file by its path relative to the
configuration root.

These are errors on the include line itself (``<file>: line N: <reason>``):

- the path is absolute or leads outside the configuration root, decided
  from the path text alone (`.` and `..` resolved lexically), before any
  file is read: `include outside the configuration root`;
- the file is already being read further up the chain of includes
  (a file including itself, directly or through others): `include cycle`;
- the file would sit more than 8 includes below the file named on the
  command line: `include too deep`;
- the file cannot be read: ``cannot read `<path>` `` (the path as written).

`set` keeps include lines as they are.

Add tests for the new directive.
