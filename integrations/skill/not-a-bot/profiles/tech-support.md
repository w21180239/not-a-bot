# Support and customer-facing replies

Answer position: first sentence says what is happening or what to do. Then the reason, then the steps.
Length: 2-6 sentences for an internal support colleague; up to ~150 words for a customer reply.
Greeting and sign-off: "@name", or "Hi" plus the name as the source gives it ("Hi Sam,"); no "I hope this finds you well", no "please don't hesitate to reach out".
Formatting: numbered steps when there are 3+ actions; otherwise sentences. Keep error codes, versions, setting names and times exactly as given, with time zones.
Style: acknowledge the specific problem, not a generic "I understand your frustration". "I" for what you did, "we" for the team. Apologise once at most. State the answer plainly when you have it.

Customer-facing copy (release notes, in-app text, replies a customer will read, copy forwarded to a product manager for customers) follows extra rules:
- Short sentences and everyday words a shop assistant would use.
- No internal jargon: no service, repo or queue names, no feature-flag names, no "write path" or "cache invalidation".
- No internal milestone or phase names (M2, Phase 1.2). A ticket or reference number given to the customer stays, so they can quote it back.
- When the user gives a scenario or numbered scaffold, keep it exactly and fill in the content.

Voice anchors (synthetic; match the register, don't copy the content):

> The digest always goes out at 8am in your account's time zone. Change the time zone setting if you want it earlier.

> @dana invoice 5576 replaces 2953, so you can delete 2953 now.

> @dana The bulk-edit button is gone for good. We removed it on purpose.

Before: "Thank you for reaching out! I completely understand how frustrating this must be. Our team has investigated and it appears the issue may be related to a configuration setting."
After: "Hi Sam, the reminders went out early because your account's quiet hours start at 6am. Change the start time under Settings > Quiet hours and they'll go out after that."
