# Spec Delta

## REMOVED Requirements

### Requirement: Read PNG images
**Reason**: Moved to the `formats/png` capability so all PNG behavior lives in one spec.
**Migration**: See `formats/png`, requirement "Read PNG images" (unchanged text and scenarios).

### Requirement: Read BMP images
**Reason**: Moved to the `formats/bmp` capability so all BMP behavior lives in one spec.
**Migration**: See `formats/bmp`, requirement "Read BMP images" (unchanged text and scenarios).

### Requirement: Maximum image size
**Reason**: The size limit applies to every storage format, so it moved to the `storage-formats` framework capability.
**Migration**: See `storage-formats`, requirement "Maximum image size" (same limit, error and scenarios, stated for every registered format).
