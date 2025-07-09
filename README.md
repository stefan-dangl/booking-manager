# booking-manager
A simple full stack booking tool written in Rust


### How to setup the Database

- Install Postgres Database and Diesel
- Only Postgres supported for now
- adapt `.env` file according to your database
- run migration: `diesel migration run`

- adapt `booking_manager/diesel.toml`:
    - `dir = "~/personal_repos/rust/booking-manager/booking_manager/migrations"` has to be adapted according to your system