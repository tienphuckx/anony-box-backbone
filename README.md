
# Backbone API for Anonymous ChatBox
Backend API for an anonymous chat application.

## Steps to Apply Database Modifications

### 1. Generate a New Migration with `diesel_cli`
To create a new migration file, use the following command:

```bash
diesel migration generate migration_name
```

This will create two SQL script files in the `migrations` folder:
- **up.sql** - Defines the logic for applying the new changes.
- **down.sql** - Defines the logic for reverting the changes.

> **Note**: Follow the naming convention `yyyy-mm-dd-description-of-change` for migration names.  
> **Example**: `2024-10-26-create-table-xyz`

### 2. Define the Migration Logic
Edit the `up.sql` and `down.sql` files to specify the SQL commands required for your changes.

### 3. Apply the Migration to the Database
Run the migration to apply changes to the database:

```bash
diesel migration run
```

### 4. Update the Database Schema in Code
To reflect the new database schema in your code, use:

```bash
diesel print-schema > ./src/database/schema.rs
```

This command will update the `schema.rs` file with the latest table definitions.

### 5. Adjust Models for Database Changes
Modify any necessary models in `./src/database/models.rs` to align with the updated schema.