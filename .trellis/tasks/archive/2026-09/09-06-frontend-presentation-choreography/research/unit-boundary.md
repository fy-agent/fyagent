# Production time-unit boundary

Material baseline on the unchanged animation implementation found the production
CSS contains `--fy-motion-dialog-enter: .42s` and exit `.32s`. The source has
420ms/320ms; CSS minification is equivalent for browsers, but motionDuration's
parseFloat/1000 is not unit-aware. Actual supplied durations become 0.00042s.
This is direct generated-bundle evidence, not a guessed animation preference.

Fix the existing duration boundary to accept a single nonnegative finite CSS
time in ms or s, return seconds, and fail closed on invalid/missing values.
Keep 420ms and .42s equivalent in unit tests and assert the compiled production
modal's running keyframe durations/real elapsed time. Do not disable CSS minify,
increase tokens by1000, copy a second parser into Dialog, or lower frame budgets.

The initial 20-cycle test data is retained as failed baseline evidence. Its
short cycles did not represent complete presentation animation; compare costs
only after verifying full configured durations. A separate timing-corrected
baseline may be needed to attribute material/choreography cost fairly.
