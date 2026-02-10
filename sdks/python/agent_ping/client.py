from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, List, Optional

import httpx


@dataclass
class AgentPingClient:
    base_url: str
    token: Optional[str] = None
    timeout: float = 30.0

    def _headers(self) -> Dict[str, str]:
        headers: Dict[str, str] = {}
        if self.token:
            headers["X-Agent-Ping-Token"] = self.token
        return headers

    def send_message(
        self,
        session_key: str,
        text: Optional[str] = None,
        attachments: Optional[List[Dict[str, Any]]] = None,
        channel: Optional[str] = None,
        account_id: Optional[str] = None,
        peer_id: Optional[str] = None,
        reply_to: Optional[str] = None,
    ) -> Dict[str, Any]:
        payload = {
            "session_key": session_key,
            "text": text,
            "attachments": attachments or [],
            "channel": channel,
            "account_id": account_id,
            "peer_id": peer_id,
            "reply_to": reply_to,
        }
        with httpx.Client(timeout=self.timeout) as client:
            resp = client.post(f"{self.base_url}/v1/messages/send", json=payload, headers=self._headers())
            resp.raise_for_status()
            return resp.json()

    def send_media(
        self,
        session_key: str,
        url: str,
        filename: Optional[str] = None,
        mime_type: Optional[str] = None,
        text: Optional[str] = None,
    ) -> Dict[str, Any]:
        attachments = [
            {
                "url": url,
                "filename": filename,
                "mime_type": mime_type,
            }
        ]
        return self.send_message(session_key=session_key, text=text, attachments=attachments)

    def list_sessions(self, limit: int = 100, offset: int = 0) -> List[Dict[str, Any]]:
        with httpx.Client(timeout=self.timeout) as client:
            resp = client.get(f"{self.base_url}/v1/sessions", params={"limit": limit, "offset": offset}, headers=self._headers())
            resp.raise_for_status()
            return resp.json()

    def get_messages(self, session_key: str, limit: int = 200, offset: int = 0) -> List[Dict[str, Any]]:
        with httpx.Client(timeout=self.timeout) as client:
            resp = client.get(f"{self.base_url}/v1/sessions/{session_key}/messages", params={"limit": limit, "offset": offset}, headers=self._headers())
            resp.raise_for_status()
            return resp.json()

    def emit_event(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        with httpx.Client(timeout=self.timeout) as client:
            resp = client.post(f"{self.base_url}/v1/inbound/ack", json=payload, headers=self._headers())
            resp.raise_for_status()
            return resp.json()
