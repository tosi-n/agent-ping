import express from 'express';
import morgan from 'morgan';

const app = express();
app.use(express.json({ limit: '20mb' }));
app.use(morgan('dev'));

// TODO: Replace with Baileys integration.

app.post('/send', async (req, res) => {
  const { to, text, attachments } = req.body || {};
  if (!to) {
    return res.status(400).json({ error: 'missing to' });
  }
  console.log('WhatsApp send', { to, text, attachments });
  return res.json({ status: 'ok', message_id: Date.now().toString() });
});

app.post('/inbound', async (req, res) => {
  // Placeholder: sidecar can POST normalized payload to Agent-Ping inbound endpoint.
  console.log('WhatsApp inbound', req.body);
  return res.json({ status: 'ok' });
});

const port = process.env.WHATSAPP_SIDECAR_PORT || 4040;
app.listen(port, () => {
  console.log(`WhatsApp sidecar listening on ${port}`);
});
