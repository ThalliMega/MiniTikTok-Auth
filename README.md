This is *just* a homework project using [![Rust]](https://www.rust-lang.org "Rust").

# Ports

Server listens on `[::]:14514`.

# Environment Variables

## Runtime env vars

### REDIS_URL

The URL format is `redis://[<username>][:<password>@]<hostname>[:port][/<db>]`.  
Check the [documention](https://docs.rs/redis/latest/redis/#connection-parameters) for details.

### POSTGRES_URL

Check the [documention](https://docs.rs/tokio-postgres/latest/tokio_postgres/config/struct.Config.html) for details.

#### postgres table layouts

```sql
CREATE TABLE `auth` (
	`id` INT unsigned AUTO_INCREMENT,
	`username` VARCHAR(32) NOT NULL,
	`password` VARCHAR(32) NOT NULL,
	UNIQUE KEY `username_idx` (`username`) USING HASH,
	PRIMARY KEY (`id`)
);
```

*Note: Id 0 is preserved and used as a user that does not exist.*

### RUST_LOG

Check the [documention](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for details.

### RUST_LOG_STYLE

Check the [documention](https://docs.rs/env_logger/latest/env_logger/#disabling-colors) for details.

## Buildtime env vars

When build the image, specify build args with [--build-args](https://docs.docker.com/engine/reference/commandline/build/#-set-build-time-variables---build-arg).

### REPLACE_ALPINE

This value will be passed to [sed](https://manpages.org/sed) as a script when modifying [/etc/apk/repositories](https://man.archlinux.org/man/community/apk-tools/apk-repositories.5.en).

[Rust]: https://img.shields.io/badge/Rust-ffffff?style=for-the-badge&labelColor=ffffff&logoColor=000000&logo=rust