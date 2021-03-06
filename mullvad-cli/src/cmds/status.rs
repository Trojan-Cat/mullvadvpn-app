use crate::{format::print_keygen_event, new_rpc_client, Command, Error, Result};
use mullvad_management_interface::{
    types::{
        daemon_event::Event as EventType,
        error_state::{
            firewall_policy_error::ErrorType as FirewallPolicyErrorType, Cause as ErrorStateCause,
            FirewallPolicyError, GenerationError,
        },
        ErrorState, ProxyType, TransportProtocol, TunnelEndpoint, TunnelState, TunnelType,
    },
    ManagementServiceClient,
};
use mullvad_types::auth_failed::AuthFailed;
use std::fmt::Write;

pub struct Status;

#[mullvad_management_interface::async_trait]
impl Command for Status {
    fn name(&self) -> &'static str {
        "status"
    }

    fn clap_subcommand(&self) -> clap::App<'static, 'static> {
        clap::SubCommand::with_name(self.name())
            .about("View the state of the VPN tunnel")
            .arg(
                clap::Arg::with_name("location")
                    .long("location")
                    .short("l")
                    .help("Prints the current location and IP. Based on GeoIP lookups"),
            )
            .subcommand(
                clap::SubCommand::with_name("listen")
                    .about("Listen for VPN tunnel state changes")
                    .arg(
                        clap::Arg::with_name("verbose")
                            .short("v")
                            .help("Enables verbose output"),
                    ),
            )
    }

    async fn run(&self, matches: &clap::ArgMatches<'_>) -> Result<()> {
        let mut rpc = new_rpc_client().await?;
        let state = rpc.get_tunnel_state(()).await?.into_inner();

        print_state(&state);
        if matches.is_present("location") {
            print_location(&mut rpc).await?;
        }

        if let Some(listen_matches) = matches.subcommand_matches("listen") {
            let verbose = listen_matches.is_present("verbose");

            let mut events = rpc.events_listen(()).await?.into_inner();

            while let Some(event) = events.message().await? {
                match event.event.unwrap() {
                    EventType::TunnelState(new_state) => {
                        print_state(&new_state);
                        use mullvad_management_interface::types::tunnel_state::State::*;
                        match new_state.state.unwrap() {
                            Connected(..) | Disconnected(..) => {
                                if matches.is_present("location") {
                                    print_location(&mut rpc).await?;
                                }
                            }
                            _ => {}
                        }
                    }
                    EventType::Settings(settings) => {
                        if verbose {
                            println!("New settings: {:#?}", settings);
                        }
                    }
                    EventType::RelayList(relay_list) => {
                        if verbose {
                            println!("New relay list: {:#?}", relay_list);
                        }
                    }
                    EventType::VersionInfo(app_version_info) => {
                        if verbose {
                            println!("New app version info: {:#?}", app_version_info);
                        }
                    }
                    EventType::KeyEvent(key_event) => {
                        if verbose {
                            print!("Key event: ");
                            print_keygen_event(&key_event);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn print_state(state: &TunnelState) {
    use mullvad_management_interface::types::{tunnel_state, tunnel_state::State::*};

    print!("Tunnel status: ");
    match state.state.as_ref().unwrap() {
        Error(error) => print_error_state(error.error_state.as_ref().unwrap()),
        Connected(tunnel_state::Connected { relay_info }) => {
            let endpoint = relay_info
                .as_ref()
                .unwrap()
                .tunnel_endpoint
                .as_ref()
                .unwrap();
            println!("Connected to {}", format_endpoint(&endpoint));
        }
        Connecting(tunnel_state::Connecting { relay_info }) => {
            let endpoint = relay_info
                .as_ref()
                .unwrap()
                .tunnel_endpoint
                .as_ref()
                .unwrap();
            println!("Connecting to {}...", format_endpoint(&endpoint));
        }
        Disconnected(_) => println!("Disconnected"),
        Disconnecting(_) => println!("Disconnecting..."),
    }
}

fn format_endpoint(endpoint: &TunnelEndpoint) -> String {
    let mut out = format!(
        "{} {} over {}",
        match TunnelType::from_i32(endpoint.tunnel_type).expect("unknown tunnel protocol") {
            TunnelType::Wireguard => "WireGuard",
            TunnelType::Openvpn => "OpenVPN",
        },
        endpoint.address,
        format_protocol(
            TransportProtocol::from_i32(endpoint.protocol).expect("unknown transport protocol")
        ),
    );

    if let Some(ref proxy) = endpoint.proxy {
        write!(
            &mut out,
            " via {} {} over {}",
            match ProxyType::from_i32(proxy.proxy_type).expect("unknown proxy type") {
                ProxyType::Shadowsocks => "Shadowsocks",
                ProxyType::Custom => "custom bridge",
            },
            proxy.address,
            format_protocol(
                TransportProtocol::from_i32(proxy.protocol).expect("unknown transport protocol")
            ),
        )
        .unwrap();
    }

    out
}

fn print_error_state(error_state: &ErrorState) {
    if error_state.blocking_error.is_some() {
        eprintln!("Mullvad daemon failed to setup firewall rules!");
        eprintln!("Deamon cannot block traffic from flowing, non-local traffic will leak");
    }

    match ErrorStateCause::from_i32(error_state.cause) {
        Some(ErrorStateCause::AuthFailed) => {
            println!(
                "Blocked: {}",
                AuthFailed::from(error_state.auth_fail_reason.as_ref())
            );
        }
        #[cfg(target_os = "linux")]
        Some(ErrorStateCause::SetFirewallPolicyError) => {
            println!("Blocked: {}", error_state_to_string(error_state));
            println!("Your kernel might be terribly out of date or missing nftables");
        }
        _ => println!("Blocked: {}", error_state_to_string(error_state)),
    }
}

fn error_state_to_string(error_state: &ErrorState) -> String {
    use ErrorStateCause::*;

    let error_str = match ErrorStateCause::from_i32(error_state.cause).expect("unknown error cause")
    {
        AuthFailed => {
            return if error_state.auth_fail_reason.is_empty() {
                "Authentication with remote server failed".to_string()
            } else {
                format!(
                    "Authentication with remote server failed: {}",
                    error_state.auth_fail_reason
                )
            };
        }
        Ipv6Unavailable => "Failed to configure IPv6 because it's disabled in the platform",
        SetFirewallPolicyError => {
            return policy_error_to_string(error_state.policy_error.as_ref().unwrap())
        }
        SetDnsError => "Failed to set system DNS server",
        StartTunnelError => "Failed to start connection to remote server",
        TunnelParameterError => {
            return format!(
                "Failure to generate tunnel parameters: {}",
                tunnel_parameter_error_to_string(error_state.parameter_error)
            );
        }
        IsOffline => "This device is offline, no tunnels can be established",
        TapAdapterProblem => "A problem with the TAP adapter has been detected",
        #[cfg(target_os = "android")]
        VpnPermissionDenied => "The Android VPN permission was denied when creating the tunnel",
        #[cfg(not(target_os = "android"))]
        _ => unreachable!("unknown error cause"),
    };

    error_str.to_string()
}

fn tunnel_parameter_error_to_string(parameter_error: i32) -> &'static str {
    match GenerationError::from_i32(parameter_error).expect("unknown generation error") {
        GenerationError::NoMatchingRelay => "Failure to select a matching tunnel relay",
        GenerationError::NoMatchingBridgeRelay => "Failure to select a matching bridge relay",
        GenerationError::NoWireguardKey => "No wireguard key available",
        GenerationError::CustomTunnelHostResolutionError => {
            "Can't resolve hostname for custom tunnel host"
        }
    }
}

fn policy_error_to_string(policy_error: &FirewallPolicyError) -> String {
    let cause = match FirewallPolicyErrorType::from_i32(policy_error.r#type)
        .expect("unknown policy error")
    {
        FirewallPolicyErrorType::Generic => return "Failed to set firewall policy".to_string(),
        FirewallPolicyErrorType::Locked => format!(
            "An application prevented the firewall policy from being set: {} (pid {})",
            policy_error.lock_name, policy_error.lock_pid
        ),
    };
    format!("Failed to set firewall policy: {}", cause)
}

async fn print_location(rpc: &mut ManagementServiceClient) -> Result<()> {
    let location = rpc.get_current_location(()).await;
    let location = match location {
        Ok(response) => response.into_inner(),
        Err(status) => {
            if status.code() == mullvad_management_interface::Code::NotFound {
                println!("Location data unavailable");
                return Ok(());
            } else {
                return Err(Error::GrpcClientError(status));
            }
        }
    };
    if !location.hostname.is_empty() {
        println!("Relay: {}", location.hostname);
    }
    if !location.ipv4.is_empty() {
        println!("IPv4: {}", location.ipv4);
    }
    if !location.ipv6.is_empty() {
        println!("IPv6: {}", location.ipv6);
    }

    print!("Location: ");
    if !location.city.is_empty() {
        print!("{}, ", location.city);
    }
    println!("{}", location.country);

    println!(
        "Position: {:.5}°N, {:.5}°W",
        location.latitude, location.longitude
    );
    Ok(())
}

fn format_protocol(protocol: TransportProtocol) -> &'static str {
    match protocol {
        TransportProtocol::Udp => "UDP",
        TransportProtocol::Tcp => "TCP",
    }
}
