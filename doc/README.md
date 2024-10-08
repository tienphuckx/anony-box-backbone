### **Anonymous Room Chat**
An anonymous chatting web

### **Mission**
Creating an anonymous chat application where users can communicate without identity, stored data, authentication, or authorization can be both interesting and challenging, especially in terms of privacy and security.

### Features
#### **Temporary Rooms/Channels**:
- Users can create chat rooms without requiring user accounts.
- Users must verify their identity via email or phone to create a room.
- Rooms are created with a unique code and can be password-protected (optional).
- Users can join a chat room using the room code and password (if applicable).
- Room admins are notified when a user requests to join, and they must approve the request.
- Room admins have the ability to remove any member from the room.
- Room admins can mute all members or mute individual members as needed.

#### **No Data Storage**:
- Chat messages are not stored beyond the session; no history is maintained.
- When the room admin ends the chat, all chat data is permanently deleted.

#### **Session-based Nicknames**:
- When a user creates a room, the session is stored and linked to the room ID, allowing others to join.
- Temporary nicknames, either random or user-chosen, are assigned for each session.
- All session-related data (including nicknames) is cleared when the room ends.

#### **Encryption**:
- All communication is encrypted to maintain privacy during the chat session.

#### **Advanced Features**:
- Chat rooms have a time limit.
- Room admins can extend the chat room’s service time by making a payment.
- If a new room is created after the time expires, all previous chat data is lost.
- Payments can be made for transferring large files, faster data transfers, or high-quality data transfers.
