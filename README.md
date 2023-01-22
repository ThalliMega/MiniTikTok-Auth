This is *just* a homework project using [![Rust]](https://www.rust-lang.org "Rust").

# Environment Variables

## REDIS_URL

The URL format is `redis://[<username>][:<password>@]<hostname>[:port][/<db>]`.

## POSTGRES_URL

Check the [documention](https://docs.rs/tokio-postgres/latest/tokio_postgres/config/struct.Config.html) for details.

### postgres table layouts

```sql
CREATE TABLE `auth` (
	`id` INT(32) unsigned AUTO_INCREMENT,
	`username` VARCHAR(20) NOT NULL CHARACTER SET utf8 COLLATE utf8_bin,
	`password` VARCHAR(20) NOT NULL CHARACTER SET utf8 COLLATE utf8_bin,
	UNIQUE KEY `username_idx` (`username`) USING HASH,
	PRIMARY KEY (`id`)
);
```

[Rust]: https://img.shields.io/badge/Rust-ffffff?style=for-the-badge&labelColor=ffffff&logoColor=000000&logo=rust