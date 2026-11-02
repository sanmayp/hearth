---
id: REQ-007
title: 'Module 4: Inventory of everything'
status: built
owner_word: '2026-10-31 — "Inventory of everything: items, locations (rooms from the LiDAR floor plan in `lidar_inventory`)" (Part B §4.4)'
modules: [inventory]
acceptance:
  - 'Items with locations; rooms seeded from the LiDAR floor plan in `lidar_inventory`'
  - 'Photos per item; barcode/QR scan on phone'
  - 'Receipts and warranties auto-linked from emailed PDF receipts'
  - 'Consumables with reorder levels; vehicles; computers seeded from `desk-setup`; documents as pointers only (files stay where they are)'
  - '"Where is my ___" search returns the item''s location'
  - 'MVP scope is manual entry and scan only (see REQ-015)'
---

Source: Part B §4.4. Build order: 4th.

## Notes

- Seeds: `lidar_inventory` (rooms/floor plan), `desk-setup` (computers).
- MVP ships manual entry + scan; receipts auto-link, reorder levels,
  and full search depth follow after MVP.
