//! Agent communication via message passing.
//!
//! Agents communicate through a simple message system backed by storage.

use serde::{Deserialize, Serialize};

/// Type of agent message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request for analysis or action.
    Request,
    /// Response to a request.
    Response,
    /// Information sharing.
    Info,
    /// Challenge to another agent's conclusion.
    Challenge,
    /// Synthesis or summary.
    Synthesis,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
            Self::Info => write!(f, "info"),
            Self::Challenge => write!(f, "challenge"),
            Self::Synthesis => write!(f, "synthesis"),
        }
    }
}

/// A message between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID.
    pub id: String,
    /// Session context.
    pub session_id: String,
    /// Sending agent ID.
    pub from_agent: String,
    /// Receiving agent ID (None = broadcast).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_agent: Option<String>,
    /// Message content.
    pub content: String,
    /// Message type.
    pub message_type: MessageType,
}

impl AgentMessage {
    /// Create a new agent message.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        from_agent: impl Into<String>,
        content: impl Into<String>,
        message_type: MessageType,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            from_agent: from_agent.into(),
            to_agent: None,
            content: content.into(),
            message_type,
        }
    }

    /// Set the recipient.
    #[must_use]
    pub fn to(mut self, agent_id: impl Into<String>) -> Self {
        self.to_agent = Some(agent_id.into());
        self
    }

    /// Check if this message is a broadcast.
    #[must_use]
    pub fn is_broadcast(&self) -> bool {
        self.to_agent.is_none()
    }
}

/// In-memory mailbox for agent communication within a session.
#[derive(Debug, Default)]
pub struct AgentMailbox {
    messages: Vec<AgentMessage>,
}

impl AgentMailbox {
    /// Create a new empty mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Send a message.
    pub fn send(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }

    /// Get messages for a specific agent.
    #[must_use]
    pub fn messages_for(&self, agent_id: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.to_agent.as_deref() == Some(agent_id) || m.is_broadcast())
            .collect()
    }

    /// Get all messages in the mailbox.
    #[must_use]
    pub fn all_messages(&self) -> &[AgentMessage] {
        &self.messages
    }

    /// Get message count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if mailbox is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::Request.to_string(), "request");
        assert_eq!(MessageType::Response.to_string(), "response");
        assert_eq!(MessageType::Info.to_string(), "info");
        assert_eq!(MessageType::Challenge.to_string(), "challenge");
        assert_eq!(MessageType::Synthesis.to_string(), "synthesis");
    }

    #[test]
    fn test_agent_message_new() {
        let msg = AgentMessage::new("m1", "s1", "analyst", "content", MessageType::Info);
        assert_eq!(msg.id, "m1");
        assert_eq!(msg.from_agent, "analyst");
        assert!(msg.is_broadcast());
    }

    #[test]
    fn test_agent_message_to() {
        let msg = AgentMessage::new("m1", "s1", "analyst", "content", MessageType::Request)
            .to("explorer");
        assert!(!msg.is_broadcast());
        assert_eq!(msg.to_agent, Some("explorer".to_string()));
    }

    #[test]
    fn test_agent_message_serialize() {
        let msg = AgentMessage::new("m1", "s1", "analyst", "test", MessageType::Challenge);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"challenge\""));
    }

    #[test]
    fn test_mailbox_new() {
        let mailbox = AgentMailbox::new();
        assert!(mailbox.is_empty());
        assert_eq!(mailbox.len(), 0);
    }

    #[test]
    fn test_mailbox_send_and_receive() {
        let mut mailbox = AgentMailbox::new();

        mailbox.send(
            AgentMessage::new("m1", "s1", "analyst", "hello", MessageType::Info).to("explorer"),
        );
        mailbox.send(AgentMessage::new(
            "m2",
            "s1",
            "analyst",
            "broadcast",
            MessageType::Info,
        ));

        assert_eq!(mailbox.len(), 2);

        let explorer_msgs = mailbox.messages_for("explorer");
        assert_eq!(explorer_msgs.len(), 2); // direct + broadcast

        let strategist_msgs = mailbox.messages_for("strategist");
        assert_eq!(strategist_msgs.len(), 1); // broadcast only
    }

    #[test]
    fn test_mailbox_all_messages() {
        let mut mailbox = AgentMailbox::new();
        mailbox.send(AgentMessage::new("m1", "s1", "a", "c", MessageType::Info));
        assert_eq!(mailbox.all_messages().len(), 1);
    }
}
