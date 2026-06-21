# Fillout to Brevo Webhook

This is a Rust application that serves as a webhook endpoint for [Fillout](https://www.fillout.com/). It automatically creates or updates a contact in [Brevo](https://www.brevo.com/) with a generated unique referral code based on their name.

## Features

- **Receives Webhook Payload:** Listens for form submission data from Fillout.
- **Generates Referral Code:** Extracts the first 3 letters of the submitted "Name" field, appends 4 random numbers, and creates a unique referral code.
- **Syncs with Brevo:** Uses the Brevo API to add the contact, saving their `email`, `NAME` and generated `REFERRAL_CODE`.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) installed.
- A **Fillout** account and a form with at least a `Name` and an `Email` field.
- A **Brevo** account with an active API Key. In your Brevo settings, ensure you have the contact attributes `NAME` and `REFERRAL_CODE` created (as text fields).

## Setup

### 1. Brevo Configuration
Before running the application, make sure your Brevo account is ready:
1. Go to your Brevo Dashboard -> **Contacts** -> **Settings** -> **Contact Attributes**.
2. Add a new normal text attribute named `NAME` (if it doesn't already exist).
3. Add another new normal text attribute named `REFERRAL_CODE`.

### 2. Local Environment
1. Clone or navigate to the project directory:
   ```bash
   cd fillout_brevo_webhook
   ```

2. Create a `.env` file in the root of the project with your API keys:
   ```env
   BREVO_API_KEY=your_brevo_api_key_here
   BREVO_LIST_ID=2
   WEBHOOK_API_KEY=your_secret_passphrase_here
   ```
   *(Note: `BREVO_LIST_ID` and `WEBHOOK_API_KEY` are optional but recommended. `WEBHOOK_API_KEY` will require Fillout to send this key in the `Authorization` or `X-API-Key` header.)*

3. Run the application:
   ```bash
   cargo run
   ```
   The server will start listening on `0.0.0.0:3050`.

### 3. Exposing to the Internet (for Local Testing)
Webhooks require a public URL to receive data. If you are testing locally, you can use [ngrok](https://ngrok.com/) to expose your local port:

```bash
ngrok http 3050
```
Copy the generated Forwarding URL (e.g., `https://abc-123.ngrok-free.app`).

## Webhook Configuration

In your Fillout form settings:
1. Go to **Integrations** > **Webhooks**.
2. Add a new webhook.
3. Set the Endpoint URL to your public server's address (or your ngrok URL) followed by `/webhook` (e.g., `https://abc-123.ngrok-free.app/webhook`).
4. Ensure the form has short answer fields named `Name` and `Email` (case sensitive).

## API Payload

The webhook handler expects Fillout's standard JSON format, typically resembling:

```json
{
  "fields": [
    {
      "name": "Name",
      "value": "John Doe"
    },
    {
      "name": "Email",
      "value": "john@example.com"
    }
  ]
}
```

The application processes this and sends the following body to Brevo:

```json
{
  "email": "john@example.com",
  "attributes": {
    "NAME": "John Doe",
    "REFERRAL_CODE": "JOH1234"
  }
}
```
