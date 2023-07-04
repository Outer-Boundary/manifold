# Manifold Backend

This is the backend for Manifold.

---

## Setup

### .env file

Create a `.env` file in the root of the project and populate it using the template below:

```
MANIFOLD__DATABASE__USERNAME=...
MANIFOLD__DATABASE__PASSWORD=...
MANIFOLD__DATABASE__HOST=...
MANIFOLD__DATABASE__PORT=...
MANIFOLD__DATABASE__DB_NAME=...

# This is required for the compile-time checking done by sqlx
DATABASE_URL=mysql://${MANIFOLD__DATABASE__USERNAME}:${MANIFOLD__DATABASE__PASSWORD}@${MANIFOLD__DATABASE__HOST}:${MANIFOLD__DATABASE__PORT}/${MANIFOLD__DATABASE__DB_NAME}
```

The `...` represent values that you need to fill in yourself.

### settings/development.yaml file

Copy the `production.yaml` file and rename to `development.yaml`. Update the values to match your development environment.

### pnpm

To install pnpm, first install the latest LTS version of Node, and then run the following commands:

`corepack enable`

`corepack prepare pnpm@latest --activate`

This will enable you to use pnpm as your node package manager.

### Local MySQL Database

To set up a local MySQL database to develop with, follow the installation instructions listed [here](https://dev.mysql.com/doc/mysql-installation-excerpt/8.0/en/).

Ensure that you select the `Developer Default` option when selecting the installation presets.

### Argonautica

`argonautica` is the cryptography package we use for password hashing. To be able to use the crate, a C compiler is needed
to compile the C implementation of Argon2. Follow the instructions
[here](https://docs.rs/argonautica/latest/argonautica/#installation) to set up your environment to be able to use
`argonautica`.

Ensure that you restart your system after installation to ensure that the `argonautica` crate can find the installed C
compiler.

### Redis

Redis is used to store session information for each user. To install, first follow the official Redis instructions for
installation on Windows: https://redis.io/docs/getting-started/installation/install-redis-on-windows/. Once this is
done, continue with the following:

1. In a terminal, access your ubuntu WSL container.

```
ubuntu
```

2. Stop the redis service.

```
sudo service redis-server stop
```

3. Edit your redis config (`sudo nano /etc/redis/redis.conf`) to match the following lines below:

```
...
bind 127.0.0.1 -::1 -> # bind 127.0.0.1 -::1 (Comment out the line)
...
protected-mode yes -> protected-mode no (Set to the value 'no')
...
```

4. Get net tools and run `ifconfig`, note down the **inet** value under **eth0**.

```
sudo apt install net-tools
ifconfig
```

5. Restart your redis server.

```
sudo service redis-server start
```

6. Add the IP address you saved before as the IP address of your redis connection in your `development.yaml` file.

Note that the redis connection may change after a system restart, as the IP address is dynamic. If the
connection fails, try getting the IP address from the WSL ubuntu container and updating your redis url to this address. Otherwise, a simple `sudo service redis-server restart` should get the redis server back to working again. You may need to keep the terminal open to ensure the redis server stays active.

---

## Database

### Production Database

We use planetscale as our production database provider. Planetscale supports branching for databases, so make sure that
any schema changes are applied to a development branch and then merged into the production database branch. This allows
any invalid schema changes to be caught before they are applied to production.

To apply migrations to a development branch, set the `DATABASE_URL` value in your `.env` file to the connection string
for the development branch. Then run `pnpm run migrations:run` and this will apply the migrations inside the
`migrations` folder to the development branch. This can not be done to the `main` database branch as it is protected
from direct schema changes.

### Local Development

When developping locally, run `pnpm run migrations:add {migration_name}` to create a new migration file inside the
`migrations` folder. This file can be used to apply schema changes to the database. Once you have finished making your
schema changes, run `pnpm run migrations:run` to apply all migrations to your local database. This will also be done
automatically every time the backend is started. The same `migrations:run` command can be used when connected to a
planetscale database branch to apply the migrations to the production server. This should be done to ensure the dev and
production environments remain consistent.

### ID

IDs should be stored as type `binary(16) PRIMARY KEY NOT NULL`

- `binary(16)`: stored as 16 unformatted bytes of data. The other option is to use the type `chars(36)` but that uses
  UTF8 character encoding, which has a variable byte length for each character, so at minimum the amount of bytes used
  would be 36 but it could go up to 144 bytes. There may be slightly more overhead with having to transfer UUID string
  into `binary(16)` and back, but there is a much smaller storage size and indexing speeds are greatly increased.
- `PRIMARY KEY`: This sets the id as the primary key value of the database. This ensures that each entry in the column
  is always unique.
- `NOT NULL`: There should never be any entry with an id where the id value is null, especially as the id is used as a
  primary key.

Optionally, you can also use `DEFAULT (uuid_to_bin(uuid(), true))`, which sets the `id` field so that it is not required to be set when inserting into the table. This will automatically generate a correctly encoded id value. You can still provide an id
value when inserting though, if that is preferred, you just need to remember to manually call `uuid_to_bin` on the
UUID string value, ensuring to pass `true` as the second parameter of the function. The `uuid_to_bin` function
converts a UUID string into its binary representation, and there is a related `bin_to_uuid` funcion which can be used
to convert the binary representation back into a UUID string. The `uuid` function generates a UUID string. The second
value in the `uuid_to_bin` function changes the position of the temporal bits in the UUID, which provides better
indexing performance in the database. Ensure that you always pass `true` as the second parameter to either the
`uuid_to_bin` or `bin_to_uuid` function.

---

## Style/Formatting Guide

### Routes

Routes should use `kebab-case` formatting, and should not contain any uppercase characters.
