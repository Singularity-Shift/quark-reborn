# AI Service Templates

This directory contains template files for the AI services. The real implementation files are gitignored to prevent accidental commits of sensitive prompting logic.

## Template Files

### 1. `prompt_template.rs` → `prompt.rs`
- **Purpose**: Main system prompt for the AI assistant
- **What to do**: Replace `[INSERT YOUR PROMPTING HERE]` with your custom system prompt
- **Rename**: `prompt_template.rs` → `prompt.rs`

### 2. `moderation/moderation_service_template.rs` → `moderation_service.rs`
- **Purpose**: AI-powered content moderation service
- **What to do**: Replace `[INSERT YOUR MODERATION PROMPTING HERE]` with your moderation logic
- **Rename**: `moderation_service_template.rs` → `moderation_service.rs`

### 3. `moderation/overrides_template.rs` → `overrides.rs`
- **Purpose**: Group-specific moderation rule overrides
- **What to do**: Replace `[INSERT YOUR OVERRIDE RULES PROMPTING HERE]` with your override logic
- **Rename**: `overrides_template.rs` → `overrides.rs`

### 4. `schedule_guard/schedule_guard_service_template.rs` → `schedule_guard_service.rs`
- **Purpose**: Validates scheduled prompts for safety
- **What to do**: Replace `[INSERT YOUR SCHEDULE GUARD PROMPTING HERE]` with your validation logic
- **Rename**: `schedule_guard_service_template.rs` → `schedule_guard_service.rs`

## How to Use

1. **Copy the template**: Copy the template file you want to use
2. **Rename it**: Remove the `_template` suffix
3. **Customize**: Replace the placeholder text with your actual prompting logic
4. **Test**: Ensure your custom logic works as expected

## Example

```bash
# Copy the prompt template
cp prompt_template.rs prompt.rs

# Edit prompt.rs and replace the placeholder
# [INSERT YOUR PROMPTING HERE] → Your actual system prompt

# The real prompt.rs file is now gitignored
```

## Important Notes

- **Never commit the real files**: They contain sensitive prompting logic and are gitignored
- **Keep templates updated**: When you modify the real files, update the corresponding templates
- **Backup your work**: Consider backing up your custom prompting logic separately
- **Test thoroughly**: Ensure your custom logic doesn't break existing functionality

## File Structure

```
ai/
├── README.md                           # This file
├── prompt_template.rs                  # Template for main prompt
├── moderation/
│   ├── moderation_service_template.rs  # Template for moderation service
│   └── overrides_template.rs          # Template for override rules
├── schedule_guard/
│   └── schedule_guard_service_template.rs # Template for schedule validation
└── [other AI service files...]
```

## Security

The real implementation files are gitignored because they may contain:
- API keys or sensitive configuration
- Custom prompting logic that could be exploited
- Business logic that should remain private
- Model-specific optimizations

Always review your custom logic for security implications before deployment.
