Add job dependencies with a new event `after <job> <dependency>`: from
then on, `<job>` may not start until `<dependency>` is `done`. A job may
have any number of dependencies; adding the same one again has no further
effect. Both names must be existing jobs.

A dependency that would make a job depend on itself, directly or through
other jobs, is refused with the reason `dependency cycle` (a state error).

`start` on a job with a dependency that is not done is the existing
`cannot start` error, and `next` skips such jobs.

When a job becomes `failed`, every job that depends on it, directly or
through other jobs, and is still pending also becomes `failed`.

Add tests.
