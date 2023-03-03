# CHANGELOG

* Removed blocking `core::Core::listen()` method. The library user must now set up a tokio runtime itself.  
  The library API changes:
  * Removed `core::Core::listen()`
  * `core::Core::listen_async()` renamed to `core::Core::listen()`
  * Removed `threads_number` field from `settings::Settings`

  The executable related changes:
  * `threads_number` field in a settings file is now ignored
  * The number of worker threads may be specified via commandline argument (see the executable help for details)

## 0.9.28

* Added support for configuring the library with multiple TLS certificates.
  API changes:
    * `settings::Settings::tunnel_tls_host_info` is renamed to `settings::Settings::tunnel_tls_hosts` and is now a vector of hosts
    * `settings::Settings::ping_tls_host_info` is renamed to `settings::Settings::ping_tls_hosts` and is now a vector of hosts
    * `settings::Settings::speed_tls_host_info` is renamed to `settings::Settings::speed_tls_hosts` and is now a vector of hosts
    * `settings::ReverseProxySettings::tls_host_info` is renamed to `settings::ReverseProxySettings::tls_hosts` and is now a vector of hosts

## 0.9.24

* Added speedtest support

## 0.9.13

* Test changelog entry please ignore
