use std::{
    io::{self, Write},
    net::IpAddr,
    time::Duration,
};

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use reqwest::{blocking::Client, StatusCode};
use uuid::Uuid;

use crate::{
    automation::{
        AutomationJobState, AutomationRequest, AutomationResult, AutomationValue, InspectNode,
        InspectRequest, InspectResult, InspectTarget, MAX_INSPECT_CHILDREN, MAX_INSPECT_DEPTH,
    },
    web::interface::{AutomationJobResponse, AutomationStatusResponse, StudioMode},
};

use super::automation::{
    automation_http_summary, build_client, decode_response, get_msgpack, poll_status, post_msgpack,
    server_url, verify_rojo_server, PollOptions, DEFAULT_ADDRESS, DEFAULT_PORT,
};

const DEFAULT_DEPTH: u8 = 1;
const DEFAULT_MAX_CHILDREN: u32 = 100;
const DEFAULT_MAX_INSTANCES: u32 = 2_000;
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const LOCAL_TIMEOUT: Duration = Duration::from_secs(50);

/// Inspect a DataModel path through a connected Prism Studio plugin.
#[derive(Debug, Parser)]
pub struct InspectCommand {
    /// DataModel path to inspect.
    pub target: String,

    /// Number of child levels to include.
    #[clap(long, default_value_t = DEFAULT_DEPTH, parse(try_from_str = parse_depth))]
    pub depth: u8,

    /// Maximum number of children included at each node.
    #[clap(
        long,
        default_value_t = DEFAULT_MAX_CHILDREN,
        parse(try_from_str = parse_max_children)
    )]
    pub max_children: u32,

    /// Include conservative, class-aware properties.
    #[clap(long)]
    pub properties: bool,

    /// Include attributes. Enabled by default.
    #[clap(long, action, default_value_t = true)]
    pub attributes: bool,

    /// Include CollectionService tags. Enabled by default.
    #[clap(long, action, default_value_t = true)]
    pub tags: bool,

    /// Emit deterministic JSON instead of human-readable output.
    #[clap(long)]
    pub json: bool,

    /// IP address of the running Rojo server.
    #[clap(long, default_value = DEFAULT_ADDRESS)]
    pub address: IpAddr,

    /// Port of the running Rojo server.
    #[clap(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
}

impl InspectCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let segments = parse_target(&self.target)?;
        let request = AutomationRequest::Inspect(InspectRequest {
            target: InspectTarget::Path { segments },
            depth: self.depth,
            max_children: self.max_children,
            max_instances: DEFAULT_MAX_INSTANCES,
            include_properties: self.properties,
            include_attributes: self.attributes,
            include_tags: self.tags,
        });
        let server_url = server_url(self.address, self.port);
        let client = build_client("Prism inspect")?;
        verify_rojo_server(&client, &server_url, "Prism inspect client")?;
        let status: AutomationStatusResponse = get_msgpack(
            &client,
            &server_url,
            "/api/automation/status",
            StatusCode::OK,
            "checking automation availability",
        )?;
        validate_automation_status(&status)?;
        let submitted: AutomationJobResponse = post_msgpack(
            &client,
            &server_url,
            "/api/automation/jobs",
            &request,
            StatusCode::CREATED,
            "submitting the inspect job",
        )?;
        if submitted.state != AutomationJobState::Pending {
            bail!("Prism returned an unexpected state for a newly submitted inspect job");
        }
        let job_id = Uuid::parse_str(&submitted.job_id)
            .context("Prism returned a malformed inspect job ID")?;
        let completed = poll_automation_job(&client, &server_url, job_id)?;
        let result = match completed.state {
            AutomationJobState::Succeeded => match completed.result {
                Some(AutomationResult::Inspect(result)) => result,
                _ => bail!("Prism returned a malformed or mismatched inspect result"),
            },
            AutomationJobState::Failed | AutomationJobState::TimedOut => {
                bail!(
                    "Prism inspect failed: {}",
                    completed
                        .error
                        .unwrap_or_else(|| "automation job failed without details".to_owned())
                )
            }
            state => bail!("Prism inspect polling ended in non-terminal state {state:?}"),
        };

        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        if self.json {
            serde_json::to_writer_pretty(&mut stdout, &result)
                .context("Could not render inspect JSON")?;
            writeln!(stdout).context("Could not write inspect JSON")?;
        } else {
            render_human(&result, &mut stdout)?;
        }
        Ok(())
    }
}

fn parse_depth(value: &str) -> Result<u8, String> {
    let value = value
        .parse::<u8>()
        .map_err(|_| "depth must be a non-negative integer".to_owned())?;
    if value > MAX_INSPECT_DEPTH {
        return Err(format!("depth must not exceed {MAX_INSPECT_DEPTH}"));
    }
    Ok(value)
}

fn parse_max_children(value: &str) -> Result<u32, String> {
    let value = value
        .parse::<u32>()
        .map_err(|_| "max-children must be a positive integer".to_owned())?;
    if value == 0 || value > MAX_INSPECT_CHILDREN {
        return Err(format!(
            "max-children must be between 1 and {MAX_INSPECT_CHILDREN}"
        ));
    }
    Ok(value)
}

fn parse_target(input: &str) -> anyhow::Result<Vec<String>> {
    if input.is_empty() {
        bail!("Inspect target must not be empty");
    }
    let bytes = input.as_bytes();
    let mut index = 0;
    let mut segments = Vec::new();
    while index < bytes.len() {
        let segment = if bytes[index] == b'"' {
            let start = index;
            index += 1;
            let mut escaped = false;
            while index < bytes.len() {
                let byte = bytes[index];
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            if index > bytes.len() || bytes.get(index.saturating_sub(1)) != Some(&b'"') {
                bail!("Inspect target contains an unterminated quoted segment");
            }
            let quoted = &input[start..index];
            let decoded: String = serde_json::from_str(quoted)
                .with_context(|| format!("Invalid quoted path segment {quoted}"))?;
            if decoded.is_empty() {
                bail!("Inspect target contains an empty path segment");
            }
            decoded
        } else {
            let start = index;
            let first = input[index..].chars().next().unwrap();
            if !(first == '_' || first.is_ascii_alphabetic()) {
                bail!("Unquoted path segments must be identifiers");
            }
            index += first.len_utf8();
            while index < bytes.len() {
                let character = input[index..].chars().next().unwrap();
                if character == '_' || character.is_ascii_alphanumeric() {
                    index += character.len_utf8();
                } else {
                    break;
                }
            }
            input[start..index].to_owned()
        };
        segments.push(segment);
        if index == bytes.len() {
            break;
        }
        if bytes[index] != b'.' {
            bail!("Inspect target contains an unexpected character at byte {index}");
        }
        index += 1;
        if index == bytes.len() {
            bail!("Inspect target must not end with a dot");
        }
    }

    normalize_root(&mut segments[0])?;
    Ok(segments)
}

fn normalize_root(root: &mut String) -> anyhow::Result<()> {
    *root = match root.as_str() {
        "game" | "DataModel" => "game".to_owned(),
        "workspace" | "Workspace" => "Workspace".to_owned(),
        "Lighting"
        | "ReplicatedStorage"
        | "ServerStorage"
        | "ServerScriptService"
        | "StarterGui"
        | "StarterPlayer"
        | "StarterPack"
        | "SoundService"
        | "Teams"
        | "TextChatService" => root.clone(),
        _ => bail!("Unsupported inspect root '{root}'"),
    };
    Ok(())
}

fn validate_automation_status(status: &AutomationStatusResponse) -> anyhow::Result<()> {
    if status.duplicate_session_detected {
        bail!("Prism detected multiple active Studio plugin sessions; close or disconnect the duplicate session");
    }
    let plugin = status
        .plugin
        .as_ref()
        .context("No Prism Studio plugin is connected for automation")?;
    if !plugin.connected {
        bail!("The Prism Studio automation session is stale or disconnected");
    }
    if plugin.automation_handler_version != status.automation_handler_version {
        bail!(
            "Prism automation handler version mismatch: server {}, plugin {}",
            status.automation_handler_version,
            plugin.automation_handler_version
        );
    }
    if !status.typed_automation_available {
        bail!("The connected Prism plugin does not provide typed automation handlers");
    }
    if plugin.studio_mode != StudioMode::Edit {
        bail!("Prism inspect is only available while Studio is in edit mode");
    }
    Ok(())
}

fn poll_automation_job(
    client: &Client,
    server_url: &str,
    job_id: Uuid,
) -> anyhow::Result<AutomationJobResponse> {
    let path = format!("/api/automation/jobs/{job_id}");
    poll_status(
        client,
        PollOptions {
            server_url,
            path: &path,
            timeout: LOCAL_TIMEOUT,
            interval: POLL_INTERVAL,
            operation: "polling the inspect job",
        },
        |response| {
            decode_response(
                response,
                StatusCode::OK,
                server_url,
                "polling the inspect job",
                automation_http_summary,
            )
        },
        |job: &AutomationJobResponse| {
            let response_id = Uuid::parse_str(&job.job_id)
                .context("Prism returned a malformed inspect job ID")?;
            if response_id != job_id {
                bail!("Prism returned status for the wrong automation job");
            }
            Ok(job.state.is_terminal())
        },
        || {
            anyhow!(
                "Timed out after {} seconds waiting for inspect job {job_id}; the server may retain it briefly",
                LOCAL_TIMEOUT.as_secs()
            )
        },
    )
}

fn render_human(result: &InspectResult, output: &mut impl Write) -> anyhow::Result<()> {
    render_node(&result.root, 0, output)?;
    if result.truncated {
        writeln!(
            output,
            "[truncated: {} after {} instances]",
            result.truncation_reason.as_deref().unwrap_or("limit"),
            result.visited_instances
        )?;
    }
    Ok(())
}

fn render_node(node: &InspectNode, depth: usize, output: &mut impl Write) -> anyhow::Result<()> {
    let indent = "  ".repeat(depth);
    let label = if depth == 0 { &node.path } else { &node.name };
    writeln!(output, "{indent}{label} [{}]", node.class_name)?;
    if !node.properties.is_empty() {
        writeln!(output, "{indent}  properties:")?;
        for (key, value) in &node.properties {
            writeln!(output, "{indent}    {key} = {}", render_value(value))?;
        }
    }
    if !node.attributes.is_empty() {
        writeln!(output, "{indent}  attributes:")?;
        for (key, value) in &node.attributes {
            writeln!(output, "{indent}    {key} = {}", render_value(value))?;
        }
    }
    if !node.tags.is_empty() {
        writeln!(output, "{indent}  tags:")?;
        for tag in &node.tags {
            writeln!(output, "{indent}    - {tag}")?;
        }
    }
    if !node.children.is_empty() {
        writeln!(output, "{indent}  children:")?;
        for child in &node.children {
            render_node(child, depth + 2, output)?;
        }
    }
    if node.truncated {
        writeln!(output, "{indent}  [children truncated]")?;
    }
    Ok(())
}

fn render_value(value: &AutomationValue) -> String {
    match value {
        AutomationValue::Nil => "nil".to_owned(),
        AutomationValue::Boolean { value } => value.to_string(),
        AutomationValue::Number { value } => value.to_string(),
        AutomationValue::String { value } => serde_json::to_string(value).unwrap(),
        AutomationValue::Vector2 { x, y } => format!("Vector2({x}, {y})"),
        AutomationValue::Vector3 { x, y, z } => format!("Vector3({x}, {y}, {z})"),
        AutomationValue::Color3 { r, g, b } => format!("Color3({r}, {g}, {b})"),
        AutomationValue::CFrame { components } => format!("CFrame({})", join_numbers(components)),
        AutomationValue::UDim { scale, offset } => format!("UDim({scale}, {offset})"),
        AutomationValue::UDim2 { x, y } => format!(
            "UDim2({}, {}, {}, {})",
            x.scale, x.offset, y.scale, y.offset
        ),
        AutomationValue::Rect { min, max } => {
            format!("Rect({}, {}, {}, {})", min.x, min.y, max.x, max.y)
        }
        AutomationValue::EnumItem { enum_type, name } => format!("Enum.{enum_type}.{name}"),
        AutomationValue::BrickColor { name, .. } => format!("BrickColor({name:?})"),
        AutomationValue::NumberRange { min, max } => format!("NumberRange({min}, {max})"),
        AutomationValue::InstanceReference { value } => {
            format!("{} [{}]", value.path, value.class_name)
        }
        AutomationValue::Diagnostic { error } => format!("<unavailable: {error}>"),
        AutomationValue::Array { .. } | AutomationValue::Map { .. } => {
            serde_json::to_string(value).unwrap_or_else(|_| "<invalid value>".to_owned())
        }
    }
}

fn join_numbers(values: &[f64]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Options;
    use std::collections::BTreeMap;

    #[test]
    fn parses_command_defaults_and_custom_values() {
        let options = Options::try_parse_from(["prism", "inspect", "Workspace"]).unwrap();
        let crate::cli::Subcommand::Inspect(command) = options.subcommand else {
            panic!("expected inspect command")
        };
        assert_eq!(command.depth, 1);
        assert_eq!(command.max_children, 100);
        assert_eq!(command.address, "127.0.0.1".parse::<IpAddr>().unwrap());
        assert_eq!(command.port, 34872);
        assert!(command.attributes);
        assert!(command.tags);

        let options = Options::try_parse_from([
            "prism",
            "inspect",
            "Workspace",
            "--depth",
            "3",
            "--max-children",
            "25",
            "--address",
            "127.0.0.2",
            "--port",
            "4000",
            "--properties",
            "--json",
        ])
        .unwrap();
        let crate::cli::Subcommand::Inspect(command) = options.subcommand else {
            panic!("expected inspect command")
        };
        assert_eq!(command.depth, 3);
        assert_eq!(command.max_children, 25);
        assert!(command.properties);
        assert!(command.json);
    }

    #[test]
    fn validates_depth_and_child_limits() {
        assert!(
            Options::try_parse_from(["prism", "inspect", "Workspace", "--depth", "9"]).is_err()
        );
        assert!(
            Options::try_parse_from(["prism", "inspect", "Workspace", "--max-children", "0"])
                .is_err()
        );
    }

    #[test]
    fn parses_target_grammar_and_escapes() {
        assert_eq!(parse_target("workspace").unwrap(), vec!["Workspace"]);
        assert_eq!(
            parse_target("Workspace.Map").unwrap(),
            vec!["Workspace", "Map"]
        );
        assert_eq!(
            parse_target("Workspace.\"Name.With.Dots\"").unwrap(),
            vec!["Workspace", "Name.With.Dots"]
        );
        assert_eq!(
            parse_target("Workspace.\"Quote \\\" Name\"").unwrap(),
            vec!["Workspace", "Quote \" Name"]
        );
        assert_eq!(
            parse_target("Workspace.\"Backslash \\\\ Name\"").unwrap(),
            vec!["Workspace", "Backslash \\ Name"]
        );
    }

    #[test]
    fn rejects_malformed_or_expression_like_targets() {
        for target in [
            "Workspace.",
            "Workspace..Map",
            "Workspace[\"Map\"]",
            "Workspace/Map",
            "Workspace.\"bad\\q\"",
            "NotAService.Map",
            "inspect.lua",
        ] {
            assert!(parse_target(target).is_err(), "accepted {target}");
        }
    }

    #[test]
    fn rejects_missing_target() {
        assert!(Options::try_parse_from(["prism", "inspect"]).is_err());
    }

    fn sample_result() -> InspectResult {
        InspectResult {
            root: InspectNode {
                reference: crate::automation::InstanceReference {
                    session_id: "plugin-session".to_owned(),
                    id: "pinst-00000001".to_owned(),
                    path: "Workspace.Map".to_owned(),
                    name: "Map".to_owned(),
                    class_name: "Model".to_owned(),
                },
                name: "Map".to_owned(),
                class_name: "Model".to_owned(),
                path: "Workspace.Map".to_owned(),
                properties: BTreeMap::from([(
                    "Archivable".to_owned(),
                    AutomationValue::Boolean { value: true },
                )]),
                attributes: BTreeMap::from([(
                    "Theme".to_owned(),
                    AutomationValue::String {
                        value: "Night".to_owned(),
                    },
                )]),
                tags: vec!["Generated".to_owned()],
                children: Vec::new(),
                truncated: true,
            },
            visited_instances: 1,
            truncated: true,
            truncation_reason: Some("maxChildren".to_owned()),
        }
    }

    #[test]
    fn renders_human_and_json_output() {
        let result = sample_result();
        let mut output = Vec::new();
        render_human(&result, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Workspace.Map [Model]"));
        assert!(output.contains("Theme = \"Night\""));
        assert!(output.contains("[truncated:"));

        let json = serde_json::to_string(&result).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded["root"]["reference"]["sessionId"], "plugin-session");
    }

    #[test]
    fn reports_status_failures() {
        let mut status = AutomationStatusResponse {
            server_session_id: crate::SessionId::new(),
            server_version: "test".to_owned(),
            protocol_version: 5,
            automation_handler_version: 2,
            automation_available: false,
            exec_available: false,
            typed_automation_available: false,
            plugin: None,
            duplicate_session_detected: false,
            queues: crate::web_api::AutomationQueueStatusResponse {
                exec_pending: 0,
                exec_claimed: 0,
                exec_claimed_by_plugin_session_id: None,
                automation_pending: 0,
                automation_claimed: 0,
                automation_claimed_by_plugin_session_id: None,
            },
        };
        assert!(validate_automation_status(&status).is_err());
        status.duplicate_session_detected = true;
        assert!(validate_automation_status(&status)
            .unwrap_err()
            .to_string()
            .contains("multiple"));
    }
}
