mod query;
pub use query::query_services;

mod register;
pub use register::register_service;

mod daemon;
pub use daemon::run_daemon;

#[cfg(test)]
mod tests {
    use gethostname::gethostname;

    #[test]
    fn test_register_service() {
        println!("{}", gethostname().to_string_lossy());
    }
}
