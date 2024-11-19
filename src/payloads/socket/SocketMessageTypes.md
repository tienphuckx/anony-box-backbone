# Socket Message Types's Structure


**SMessageType::Authenticate JSON:**

Before a client can send or receive messages from a group, it must authenticate itself to the server using a unique authentication token.

```json
{
    "Authenticate": "C9D053830E6A88E565B3A85480A391D9C180CDC48F5313140CD4A1C73223B640"
}
```

---

**SMessageType::AuthenticateResponse JSON:**

After the client sends an authentication message, the server responds with an authentication result containing a status code and message.

- `status_code`: Represents the result of the authentication attempt
  - 0: Authentication successful
  - 1: Authentication timed out
  - 2: Unsupported authentication message type
  - 3: User lacks permission to access the group
  - 4: User token is expired or invalid
  - 5: Failed to retrieve user based on provided credentials
- `message`: A short message to explain the result

```json
{
  "AuthenticateResponse": {
    "status_code": 0,
    "message": "Authenticated Successfully"
  }
}
```

---

**SMessageType::Send JSON:**

Structure of the "Send" message, used by a client to send a message to a group.

```json
{
  "Send": {
    "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
    "group_id": 24,
    "content": "Hello, World!"
  }
}
```

---

**SMessageType::Receive JSON:**

When a new message is sent to a group, the server sends a "Receive" message to all clients subscribed to that group.

```json
{
  "Receive": {
      "message_uuid": "6739e721-91af-4042-9441-2b7c832d42aa",
      "user_id": 38,
      "group_id": 25,
      "content": "Hello world",
      "created_at": "2024-11-12T07:32:25.455274+00:00",
      "status": "Sent"
  }
}
```

---

**SMessageType::Edit JSON:**

The "Edit" message structure, used by the client to modify the content of an existing message in the group.

```json
{
  "Edit": {
    "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": 1,
    "group_id": 1,
    "content": "Hello, Group!",
    "created_at": "2024-11-08T12:00:00Z",
    "status": "Sent"
  }
}
```

---

**SMessageType::DeleteMessage JSON:**

The "Delete" message structure, which specifies a list of message identifiers `message_ids` that the client requests to delete from the specific group `group_id`.

```json
{
    "DeleteMessage": {
        "group_id": 24,
        "message_ids": [
            38,39
        ]
    }
}
```
**SMessageType::DeleteMessageResponse JSON:**

After client request a delete message, if an error occurs the Delete message response will be sent from server with a short message to explain the error.
```json
{
  "DeleteMessageResponse": {
      "status_code": 2,
      "message": "Failed to delete message, maybe one of messages is not found"
  }
}
```

**SMessageType::DeleteMessageEvent JSON:**
The message will be responded from server if a delete message request was processed successfully to inform all connected client in a group.

```json
{
    "DeleteMessageEvent": {
        "group_id": 24,
        "message_ids": [
            38,
            39
        ]
    }
}
```