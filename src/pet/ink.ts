/** How the card is drawn from the tone the platform reads off the desktop.
 *
 * The tone is one number, 0 for a card drawn with dark ink over a pale panel and
 * 1 for light ink over a dark one, and it drifts with whatever the pet is
 * standing on. Two things come out of it: how much of its own colour the card
 * carries, and which ink it is drawn with.
 *
 * The first is continuous, so the panel follows the desktop a step at a time.
 * The second cannot be — dark ink and light ink have to change places somewhere,
 * and the colour between them is neither — so it is kept to the middle of the
 * band, where the two read the same, and drawn as a fade rather than a step.
 */

/** How much of its own colour the card carries at the very most, where the
    desktop behind it is too mixed to be read against. */
const BODY_MAX = 0.3;

/** How far from the middle the card has stopped carrying any colour of its own.
    Outside this the desktop is plainly one way or the other, and the panel is
    left as the glass it is. */
const BODY_SPAN = 0.55;

/** The tone the ink changes hands at, and the tone it changes back at. Between
    the two the card keeps the ink it has: the two read much the same there, and
    a tone hovering in the middle would otherwise flicker them. */
const INK_DARK_AT = 0.6;
const INK_LIGHT_AT = 0.4;

/** Which ink the card is drawn with: `true` for the light ink of a dark panel.
    `was` is the side it is on now, which is kept between the two marks. */
export function inkSide(tone: number, was: boolean): boolean {
  if (tone >= INK_DARK_AT) return true;
  if (tone <= INK_LIGHT_AT) return false;
  return was;
}

/** How much of its own colour the card carries: none at all where the desktop is
    plainly dark or plainly pale — the panel stays glass there — and the most of
    it in the middle, where a desktop too mixed to be read against is what the
    card has to answer for itself. */
export function cardBody(tone: number): number {
  const fromMiddle = Math.abs(2 * tone - 1);
  return BODY_MAX * Math.max(0, 1 - fromMiddle / BODY_SPAN);
}
