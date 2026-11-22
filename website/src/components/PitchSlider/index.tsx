import {useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode} from 'react';
import Link from '@docusaurus/Link';
import {LifecycleGraphic, ShapeGraphic, SuiteGraphic} from './graphics';
import animation from '@site/src/generated/animation.json';
import './styles.css';

const ADVANCE_MS = 7000;
const SWIPE_PX = 40;

/**
 * The numbers of slide 3 are those of website/docs/status.md (task conform, ESS 0.97.1) and
 * website/docs/limits.md; tests/landing.spec.ts fails when status.md no longer states them.
 */
const SUITE = {total: 111, core: 111, http: 31, findings: 2};

interface Slide {
  label: string;
  claim: string;
  line: string;
  href: string;
  link: string;
  graphic: ReactNode;
}

const SLIDES: Slide[] = [
  {
    label: 'What it does',
    claim: 'Holds each message until it is handled.',
    line: 'Agents in separate processes post signed messages to one service, which keeps each for its recipient until handled or expired.',
    href: '/specification/domains/inbox-messaging#message',
    link: 'The message in the specification',
    graphic: <LifecycleGraphic />,
  },
  {
    label: 'Why it is simple',
    claim: `One service, ${animation.counts.commands} commands, one storage contract.`,
    line: 'Every client talks only to the service, and a backend plugs in behind one contract and its test kit.',
    href: '/docs/#the-shape',
    link: 'The shape of the inbox',
    graphic: <ShapeGraphic />,
  },
  {
    label: 'Why it is solid',
    claim: `Held to ${SUITE.total} scenarios from its specification.`,
    line: `All ${SUITE.core} pass against the core; over HTTP ${SUITE.http} can be witnessed and pass, and two security findings stay open.`,
    href: '/docs/limits',
    link: 'The limits, findings first',
    graphic: <SuiteGraphic {...SUITE} />,
  },
];

function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(true);
  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setReduced(query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  return reduced;
}

export default function PitchSlider(): ReactNode {
  const [index, setIndex] = useState(0);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const reduced = useReducedMotion();
  const start = useRef<number | null>(null);
  const count = SLIDES.length;
  const go = useCallback((to: number) => setIndex(((to % count) + count) % count), [count]);

  useEffect(() => {
    if (reduced || hovered || focused) return undefined;
    const timer = window.setTimeout(() => go(index + 1), ADVANCE_MS);
    return () => window.clearTimeout(timer);
  }, [index, reduced, hovered, focused, go]);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      go(index + 1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      go(index - 1);
    }
  };
  const onPointerDown = (event: PointerEvent) => {
    start.current = event.clientX;
  };
  const onPointerUp = (event: PointerEvent) => {
    if (start.current === null) return;
    const moved = event.clientX - start.current;
    start.current = null;
    if (moved <= -SWIPE_PX) go(index + 1);
    else if (moved >= SWIPE_PX) go(index - 1);
  };

  return (
    <section
      className="inbox-pitch"
      data-testid="pitch"
      aria-roledescription="carousel"
      aria-label="Why the inbox"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setFocused(false);
      }}>
      <div className="inbox-pitch__card">
      <div
        className="inbox-pitch__viewport"
        data-testid="pitch-viewport"
        tabIndex={0}
        aria-label="Slides; use the left and right arrow keys"
        onKeyDown={onKeyDown}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onPointerCancel={() => {
          start.current = null;
        }}>
        <div className="inbox-pitch__track" style={{transform: `translateX(-${index * 100}%)`}} aria-live={hovered || focused || reduced ? 'polite' : 'off'}>
          {SLIDES.map((slide, i) => (
            <article
              key={slide.label}
              className="inbox-pitch__slide"
              data-testid="pitch-slide"
              role="group"
              aria-roledescription="slide"
              aria-label={`${slide.label} (${i + 1} of ${count})`}
              aria-hidden={i !== index}
              inert={i !== index}>
              <div className="inbox-pitch__copy">
                <p className="inbox-pitch__label" data-testid="pitch-label">
                  {slide.label}
                </p>
                <h2 className="inbox-pitch__claim" data-testid="pitch-claim">
                  {slide.claim}
                </h2>
                <p className="inbox-pitch__line" data-testid="pitch-line">
                  {slide.line}
                </p>
                <Link className="inbox-pitch__link" to={slide.href} data-testid="pitch-link">
                  {slide.link} <span aria-hidden="true">→</span>
                </Link>
              </div>
              <div className="inbox-pitch__graphic">{slide.graphic}</div>
            </article>
          ))}
        </div>
      </div>
      <div className="inbox-pitch__controls">
        <button type="button" className="inbox-pitch__arrow-button" aria-label="Previous slide" onClick={() => go(index - 1)}>
          ←
        </button>
        <div className="inbox-pitch__dots">
          {SLIDES.map((slide, i) => (
            <button
              key={slide.label}
              type="button"
              className="inbox-pitch__dot"
              aria-label={`Show ${slide.label}`}
              aria-current={i === index ? 'true' : undefined}
              onClick={() => go(i)}>
              <span aria-hidden="true" />
            </button>
          ))}
        </div>
        <button type="button" className="inbox-pitch__arrow-button" aria-label="Next slide" onClick={() => go(index + 1)}>
          →
        </button>
      </div>
      </div>
    </section>
  );
}
