# Atlas landing page

An extension of the existing Atlas identity, not the proposed launcher/game redesign. Visitor mode: persuade, by making the download and open-source repository immediately clear.

## Visual system

- Dark graphite ground `#101214`, warm white `#f5f4f0`, Atlas orange `#ff8a3d`, muted text `#b0b4b9`.
- Self-hosted Manrope, including Cyrillic. Large Atlas wordmark and concise copy; no pixel font, stone buttons, unverified benchmarks or social-proof counters.
- A centred first screen over the existing night landscape, with download and GitHub actions side by side. The approximately five-second introduction reveals the title and gently settles the scene; controls never wait for it. One comet passes during the introduction, then the scene rests. Reduced motion removes all animation.
- Below it: an editorial two-column introduction, a wide existing landscape, three unboxed feature descriptions, and a short download/install section.
- 1160px content measure, 48px desktop outer padding, 20px mobile padding. Controls remain visible, keyboard accessible and large enough for touch.

## Behaviour and delivery

Russian and English copy, stored language preference, automatic latest stable Windows portable release link with a verified static fallback. No analytics. Deploy only `website/public`. Documentation, scratch browser profiles and screenshots never enter the deployed artefact.

## Review

Verified in Edge at 320, 390, 768 and 1440px, in both languages: no horizontal overflow, fonts/images load, first-screen actions remain visible. Tested reduced motion, denied storage, failed GitHub requests, untrusted release URLs, language persistence, in-page navigation and a JavaScript-disabled browser. The five-second introduction stops. Desktop and mobile screenshots inspected; functional small text enlarged after the mechanical audit. The audit's download-section padding warning is a false positive: the section has 90px desktop / 54px mobile top padding. The requested no-subagent constraint means design review was performed in this thread.
