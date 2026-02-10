# Agent-Ping Python SDK

Simple HTTP client for Agent-Ping daemon.

```
from agent_ping import AgentPingClient

client = AgentPingClient("http://localhost:8091", token="changeme")
client.send_message("agent:main:main", "Hello")
```
