# Web interface

Read this before changing what a screen of a web client or operator console shows or how it responds, and before reviewing such a change or a design critique. It covers components and tokens, accessibility, interaction, public pages, design findings and the screenshots a PR shows. The architecture behind the screens is `web.md`.

## Components and tokens

- **One component per pattern, composed, never rebuilt.** A pattern on more than one screen in the same role is one `ui/` component that screens compose, never rebuild from classes or restyle; a second screen's variant is a prop. The design document names each component with its file, and a new pattern adds its component and line there in the same PR. A shared piece moves to `ui/` the second time it appears.
- **Tokens.** Colours and other design tokens are CSS variables from the design document, never literals in components; a token's value lives in one CSS file. Tailwind classes, no inline styles except computed values.

## Accessibility

- **Native elements first;** every control has an accessible name, and focus is visible.
- **Contrast.** Text meets WCAG AA, and what one must see to use a control (a field's edge, a switch's track, a pane's boundary) reaches 3:1 against what is around it (WCAG 1.4.11). A forced-colors fallback keeps every boundary and state visible when the system replaces the colours.
- **Touch targets** are at least 44 px on a touch screen, whatever the control's drawn size.
- **An icon-only control** shows a tooltip with the same text as its accessible name, on hover and keyboard focus, never on touch.
- **Every enabled clickable control shows the pointer cursor;** a disabled one does not, and has no hover state.
- **Motion** respects `prefers-reduced-motion`, Tailwind's `animate-*` utilities included, keeping its meaning without the movement.

## Interaction

- **Every action answers at once.** What can be shown done before the server answers is shown optimistically, and rolled back with a toast saying why when it fails; waiting longer than a moment shows what is happening. Nothing an end user does goes unanswered.
- **What the end user must fix shows under its control,** where they are looking, never only in a toast or at the top of a form.
- **Exits are faster than entrances,** with the durations and curves the design document names, animating only opacity and transform so motion stays on the compositor.

## Public pages

- **One component owns a page's head:** a screen never writes `document.title`, a `<meta>` or an effect on the head itself, so titles, descriptions and alternates stay consistent and testable.
- **A public page's address decides its language.** Links in another language are built by the i18n library's localizing function, never a prefix written by hand.
- **Crawl files come from the route list:** robots.txt, the sitemap and the JSON-LD are generated from the routes and their page metadata, so a new public route is one entry and nothing drifts.

## Design findings

- **A design finding cites a rule of the project's design document** (or the outside reference it adopts). A finding backed only by taste is dropped; a better outside pattern is proposed as a change to the design document, not applied on its own.

## Screenshots in the PR

- **A PR that changes an interface shows it,** so the user can judge the look without running the branch: before-and-after screenshots at phone and desktop width, in light and dark, taken with the repository's one shots tool and embedded inline. Each changed region has a numbered box on both images, and a line under each sheet says what each number is. The user confirms the look from them before the PR merges.
