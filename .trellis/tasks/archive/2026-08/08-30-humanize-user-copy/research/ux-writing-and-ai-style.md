# Research — UX writing and generic AI prose

## Question

What makes user-facing copy feel “GPT-written”, and which rules are stable
enough to persist as an engineering constraint rather than a temporary list of
fashionable tells?

## Sources reviewed

### Product and government writing guidance

- Microsoft Writing Style Guide: “make every word matter”; use a warm,
  straightforward, crisp, and helpful voice.
  <https://learn.microsoft.com/en-us/style-guide/welcome/>
- Material Design writing guidance: UI text should be clear, accurate,
  concise, simple, and direct.
  <https://m2.material.io/design/communication/writing.html>
- Material Design content-design overview and style guide: content is part of
  interaction design and should help people complete the current task.
  <https://m3.material.io/foundations/content-design/overview>
- Apple Human Interface Guidelines, Writing: words are part of the user
  experience; interface writing should be clear, conversational, and helpful.
  <https://developer.apple.com/design/human-interface-guidelines/writing>
- GOV.UK Service Manual, Writing for user interfaces: keep copy short and
  direct, use one idea per sentence, put important words first, and remove
  unnecessary framing.
  <https://www.gov.uk/service-manual/design/writing-for-user-interfaces>
- GOV.UK style guidance: prefer active voice and plain language.
  <https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/style-guides/a-to-z-style-guide/>

### Community signals

- Hacker News discussions repeatedly criticize default model prose for being
  verbose, generic, flattering, and too polished for the context.
  <https://news.ycombinator.com/item?id=48378995>
  <https://news.ycombinator.com/item?id=43758459>
- Reddit writing communities commonly flag clichés, rehashed metaphors,
  meaningless intensifiers, repeated three-part structures, and overuse of
  stock contrast templates. These are useful symptoms, not proof of AI use.
  <https://www.reddit.com/r/freelanceWriters/comments/1oofc4c/im_so_mad_that_i_have_to_change_my_writing_style/>
  <https://www.reddit.com/r/OpenAI/comments/1pptr3f/gpt52_has_turned_chatgpt_into_an_overregulated/>
- A widely shared X post identifies the template “It’s not just about X — it’s
  about Y” when Y is vaguer than X. The problem is the empty escalation, not
  the punctuation by itself.
  <https://x.com/bryanfcasey/status/2091972070196273618>

## Synthesis

“GPT-like” is not a trustworthy authorship test. Human writers also use em
dashes, parallel lists, polished grammar, and common transitions. A durable
product rule must therefore target reader harm rather than guessed origin.

The harmful patterns are:

1. **Implementation narration in the product surface.** The interface explains
   readback, projection, state machines, evidence, or internal job identity
   instead of the result and next action.
2. **Generic escalation.** A concrete feature is followed by a vaguer claim
   about transformation, empowerment, an “era”, or a broader journey without
   adding verifiable information.
3. **Mechanical symmetry.** Repeated “not only X, but Y”, three-part slogans,
   mirrored headings, and conclusion paragraphs exist for rhythm rather than
   meaning.
4. **Over-explanation.** Labels and notices restate the screen, explain the
   author's intention, or describe safeguards the user cannot act on.
5. **Abstract nouns before actions.** Copy leads with “capability”, “strategy”,
   “ecosystem”, “readiness”, “authority”, or “evidence” instead of a verb or
   observable state.
6. **Unsupported confidence.** Marketing adjectives or success language exceed
   what current code, release artifacts, or user-visible state can prove.
7. **Uniform tone across contexts.** Errors, confirmations, onboarding, README
   introductions, and technical references all receive the same polished
   promotional voice.

## Durable rules for FyAgent

- Start with the user's object or action: “选择模型”, “设置未保存”, “打开配置
  文件”, “下载桌面应用”.
- Prefer an observable result over a proof mechanism: “WorkBuddy 已启用此
  Skill”, not “已从真实配置回读”.
- In errors, say what is known, avoid guessing, and provide one useful next
  step.
- In confirmations, state the affected file/setting, whether a backup is
  created, and when the write happens.
- Keep exact technical terms only when users need them to identify an object,
  enter a value, inspect a path, or understand a real risk.
- Do not add a “bigger meaning” sentence unless it contributes a concrete,
  testable fact.
- Do not ban punctuation or phrases globally. Review the purpose of each
  sentence in context.

## Review test

For every user-visible sentence, ask:

1. What decision or action does this help the reader make?
2. Can the same meaning be stated with fewer concepts?
3. Does it expose an implementation mechanism that belongs in logs, tests, or
   developer docs?
4. Does it claim more than the product can currently prove?
5. Does the reader know what happens next?

If no useful answer exists for question 1, remove the sentence.
