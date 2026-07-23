# Encrypted history database

pH7Console bundles the SQLCipher Community Edition amalgamation through
`libsqlite3-sys` for encrypted local command-history storage.

- SQLCipher version: 4.14.0
- SQLite baseline: 3.51.3
- License: BSD-3-Clause (`LICENSE-SQLCIPHER`)
- Source: <https://github.com/sqlcipher/sqlcipher>

SQLCipher encrypts the database, full-text search index, and write-ahead log.
Its inclusion does not imply endorsement by Zetetic LLC.
