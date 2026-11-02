# Inventory module (CONTEXT.md)

Purpose: items plus rooms. MVP: manual entry plus scan.
LiDAR seed, receipts, reorder, photos follow.

Public API:
- `Inventory` owns `Module` (id "inventory").
- `GET /v1/inventory[?q=]`, `POST /v1/inventory`.
- `GET /v1/inventory/by-barcode/:code`.
- `GET /v1/inventory/rooms`.
- Store trait `InventoryStore`: `PgInventoryStore`, `MemInventoryStore`.
- Search needs all words (case free).
- Migration v1: tables `rooms`, `items`.

Invariants:
- Add creates the room when missing.
- Empty search lists all.

Gotchas:
- Rooms seed from `lidar_inventory` when it lands.
- Photo upload needs object store (later train).
