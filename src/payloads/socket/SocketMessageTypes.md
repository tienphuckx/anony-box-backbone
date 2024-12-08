# Socket Message Types's Structure

## Authentication
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
## Send message

**SMessageType::Send JSON:**

Structure of the "Send" message, used by a client to send a message to a group.

```json
{
  "Send": {
    "message_uuid": "550e8400-e29b-41d4-a716-446655440000",
    "group_id": 24,
    "content": "Hello, World!",
    "message_type": "ATTACHMENT",
    "attachments": [
      {
        "attachment_type": "TEXT",
        "url": "http://127.0.0.1:8080/files/readme.md"
      },
      {
        "attachment_type": "IMAGE",
        "url": "http://127.0.0.1:8080/files/avatar.png"
      }
    ],
  }
}
```



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
    "message_type": "ATTACHMENT",
    "attachments": [
      {
        "attachment_type": "TEXT",
        "id": 2,
        "url": "http://127.0.0.1:8080/files/readme.md"
      },
      {
        "attachment_type": "IMAGE",
        "id": 3,
        "url": "http://127.0.0.1:8080/files/avatar.png"
      }
    ],
    "status": "Sent"
  }
}
```

## Delete messages

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
---
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
---
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

## Edit message
**SMessageType::EditMessage JSON:**

The "Edit" message structure, which specifies `content`, `message_type` fields are optional, that the client requests to update specific message `message_id`.

```json
{
  "EditMessage": {
    "message_id": 42,
    "group_id": 24,
    "content": "That is edited message 42",
    "message_type": "ATTACHMENT"
  }
}
```
---
**SMessageType::EditMessageResponse JSON:**

After client request a edit message, if an error occurs a edit message response will be sent from server with a short message to explain the error.
```json
{
  "EditMessageResponse": {
    "status_code": 2,
    "message": "Failed to update message, please try again later"
  }
}
```
---
**SMessageType::EditMessageData JSON:**
The message will be responded from server if a update message request was processed successfully to inform all connected client in a group.

```json
{
  "EditMessageData": {
    "message_uuid": "adb8e186-b133-4874-b14e-5741226f68bc",
    "message_id": 42,
    "user_id": 37,
    "group_id": 24,
    "content": "That is edited message 42",
    "username": null,
    "message_type": "ATTACHMENT",
    "status": "Sent",
    "created_at": "2024-11-19T09:25:54.219284+00:00",
    "updated_at": "2024-11-19T09:26:26.979009+00:00",
    "status": "Sent"
  }
}
```
## Seen Message
**SMessageType::SeenMessages JSON:**
The `Seen` message structure which contains list of `messages_ids` that client requests to change status of message to seen
```json
{
  "SeenMessages": {
      "group_id": 24,
      "message_ids": [
          41,42
      ]
  }
}
```
---
**SMessageType::SeenMessagesResponse JSON:**
After sending a seen message, if any error occurs the seen message response will be sent from server with a short message to explain the error.
```json

{
  "SeenMessagesResponse": {
      "status_code": 4,
      "message": "One of messages is not belong to group 24"
  }
}

```
---
**SMessageType::SeenMessagesEvent JSON:**
The message will be responded from server if a seen message request was processed successfully to inform all connected client in a group.

```json
{
  "SeenMessagesEvent": {
    "group_id": 24,
    "message_ids": [
      33,
      37
    ]
  }
}
```


