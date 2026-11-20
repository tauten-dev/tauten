Let `kvconf get` select one element of a list value.

The key given to `get` may end with an element selector: `<key>[<index>]`.
Index 0 is the first element of the list, and a negative index counts from
the end, so `-1` is the last element. The selected element is printed the
same way a whole value is printed, and the type options (`--int`, `--bool`,
`--str`) apply to the element. A key without a selector behaves as before.

A selector on a key whose value is not a list is the error
``key `<key>` is not of type list``. An index that does not select an
element of the list is the error `index out of range`. If the text after
the key is not exactly `[`, an index, `]`, the whole text is a bad key
(``bad key `<text>` ``). Errors name the key without its selector.

Add tests for the new behaviour.
