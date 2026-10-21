use super::AppService;
use crate::validation;
impl AppService {
    pub fn connect_returning(&self, id: &str) -> Result<i32, String> {
        let session = self.record_attempt(id)?;
        crate::connect::SshConnection::execute_returning(&session, &[])
    }
    pub fn ping(
        &self,
        id: &str,
        icmp: bool,
        seconds: Option<u64>,
    ) -> Result<crate::connect::PingResult, String> {
        let s = self.get(id, None)?;
        let seconds = seconds.unwrap_or(self.storage.load_config()?.ping_timeout_seconds);
        validation::timeout(seconds)?;
        Ok(if icmp {
            crate::connect::PingService::ping_icmp(&s.host, seconds)
        } else {
            crate::connect::PingService::ping_tcp(
                &s.host,
                s.port,
                std::time::Duration::from_secs(seconds),
            )
        })
    }
}
