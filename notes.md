# bucket-sync/notes.md

## Output Directory Strategy

- All files and generated data will be stored within a dedicated `output` directory at the project root, e.g., `./output`.
- For batch processing of multiple sources, each source will have its own subdirectory inside `output`, named deterministically based on source type and parameters (e.g., hash or sanitized string).
- This structure allows easy organization, selective syncing, and clear separation of sources.

## Gitignore Recommendations

- The entire `output` directory should be included in `.gitignore` to prevent committing large, generated data.
- Example entry in `.gitignore`:
  ```
  /output/
  ```
- Source code and configuration files must be tracked, but outputs are considered generated and transient.

## Future Ideas & Considerations

- Extend configuration schema to derive output directory per source dynamically; e.g., by hashing source parameters or using a naming convention.
- Enhance the CLI to accept an `--output-dir` argument that overrides configuration defaults.
- Implement a mechanism to verify recent modifications (e.g., check modification timestamps) and ensure content is up-to-date.
- Add command-line features to clean or reset output directories.
- Consider versioning or checksum validation for sync integrity.

## Additional Notes

- These strategies aim to provide a clean, manageable separation between source repositories and their locally stored clones, facilitating incremental updates and avoiding conflicts.
- Proper `.gitignore` management ensures that large cloned repositories or generated content do not clutter version control history.