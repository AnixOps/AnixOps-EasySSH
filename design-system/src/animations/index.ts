import { useEffect, useState, useCallback, useRef } from 'react';

/**
 * Animation Utilities
 *
 * React hooks and utilities for smooth, accessible animations.
 * Respects reduced motion preferences automatically.
 */

// ============================================================================
// Reduced Motion Detection
// ============================================================================

export const useReducedMotion = (): boolean => {
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    setReducedMotion(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => {
      setReducedMotion(e.matches);
    };

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  return reducedMotion;
};

// ============================================================================
// Fade Animation Hook
// ============================================================================

export interface FadeOptions {
  duration?: number;
  delay?: number;
  direction?: 'in' | 'out' | 'in-out';
}

export const useFade = (visible: boolean, options: FadeOptions = {}) => {
  const { duration = 200, delay = 0, direction = 'in-out' } = options;
  const reducedMotion = useReducedMotion();
  const [state, setState] = useState<'entering' | 'entered' | 'exiting' | 'exited'>(
    visible ? 'entered' : 'exited'
  );

  useEffect(() => {
    if (reducedMotion) {
      setState(visible ? 'entered' : 'exited');
      return;
    }

    if (visible) {
      if (state === 'exited' || state === 'exiting') {
        setState('entering');
        const timer = setTimeout(() => setState('entered'), delay + duration);
        return () => clearTimeout(timer);
      }
    } else {
      if (state === 'entered' || state === 'entering') {
        setState('exiting');
        const timer = setTimeout(() => setState('exited'), duration);
        return () => clearTimeout(timer);
      }
    }
  }, [visible, duration, delay, reducedMotion, state]);

  const styles: React.CSSProperties = {
    opacity: state === 'entered' || state === 'entering' ? 1 : 0,
    transition: reducedMotion ? 'none' : `opacity ${duration}ms ease-out ${delay}ms`,
    visibility: state === 'exited' ? 'hidden' : 'visible',
  };

  return { state, styles, isVisible: state === 'entered' || state === 'entering' };
};

// ============================================================================
// Slide Animation Hook
// ============================================================================

export interface SlideOptions {
  duration?: number;
  delay?: number;
  direction?: 'up' | 'down' | 'left' | 'right';
  distance?: number;
}

export const useSlide = (visible: boolean, options: SlideOptions = {}) => {
  const {
    duration = 300,
    delay = 0,
    direction = 'up',
    distance = 20,
  } = options;
  const reducedMotion = useReducedMotion();
  const [state, setState] = useState<'entering' | 'entered' | 'exiting' | 'exited'>(
    visible ? 'entered' : 'exited'
  );

  useEffect(() => {
    if (reducedMotion) {
      setState(visible ? 'entered' : 'exited');
      return;
    }

    if (visible) {
      setState('entering');
      const timer = setTimeout(() => setState('entered'), delay + duration);
      return () => clearTimeout(timer);
    } else {
      setState('exiting');
      const timer = setTimeout(() => setState('exited'), duration);
      return () => clearTimeout(timer);
    }
  }, [visible, duration, delay, reducedMotion]);

  const getTransform = () => {
    const isVisible = state === 'entered' || state === 'entering';
    const offset = isVisible ? 0 : distance;

    switch (direction) {
      case 'up':
        return `translateY(${offset}px)`;
      case 'down':
        return `translateY(-${offset}px)`;
      case 'left':
        return `translateX(${offset}px)`;
      case 'right':
        return `translateX(-${offset}px)`;
      default:
        return 'none';
    }
  };

  const styles: React.CSSProperties = {
    transform: getTransform(),
    opacity: state === 'exited' ? 0 : 1,
    transition: reducedMotion
      ? 'none'
      : `transform ${duration}ms cubic-bezier(0.23, 1, 0.32, 1) ${delay}ms, opacity ${duration}ms ease ${delay}ms`,
    visibility: state === 'exited' ? 'hidden' : 'visible',
  };

  return { state, styles };
};

// ============================================================================
// Scale Animation Hook
// ============================================================================

export interface ScaleOptions {
  duration?: number;
  delay?: number;
  initialScale?: number;
}

export const useScale = (visible: boolean, options: ScaleOptions = {}) => {
  const { duration = 200, delay = 0, initialScale = 0.95 } = options;
  const reducedMotion = useReducedMotion();
  const [state, setState] = useState<'entering' | 'entered' | 'exiting' | 'exited'>(
    visible ? 'entered' : 'exited'
  );

  useEffect(() => {
    if (reducedMotion) {
      setState(visible ? 'entered' : 'exited');
      return;
    }

    if (visible) {
      setState('entering');
      const timer = setTimeout(() => setState('entered'), delay + duration);
      return () => clearTimeout(timer);
    } else {
      setState('exiting');
      const timer = setTimeout(() => setState('exited'), duration);
      return () => clearTimeout(timer);
    }
  }, [visible, duration, delay, reducedMotion]);

  const scale = state === 'entered' || state === 'entering' ? 1 : initialScale;

  const styles: React.CSSProperties = {
    transform: `scale(${scale})`,
    opacity: state === 'exited' ? 0 : 1,
    transition: reducedMotion
      ? 'none'
      : `transform ${duration}ms cubic-bezier(0.34, 1.56, 0.64, 1) ${delay}ms, opacity ${duration}ms ease ${delay}ms`,
    visibility: state === 'exited' ? 'hidden' : 'visible',
  };

  return { state, styles };
};

// ============================================================================
// Stagger Animation Hook
// ============================================================================

export interface StaggerOptions {
  staggerDelay?: number;
  duration?: number;
  direction?: 'up' | 'down' | 'left' | 'right' | 'fade' | 'scale';
}

export const useStagger = (itemCount: number, options: StaggerOptions = {}) => {
  const { staggerDelay = 50, duration = 300, direction = 'up' } = options;
  const reducedMotion = useReducedMotion();
  const [visibleItems, setVisibleItems] = useState<number>(0);

  useEffect(() => {
    if (reducedMotion) {
      setVisibleItems(itemCount);
      return;
    }

    let current = 0;
    const timers: NodeJS.Timeout[] = [];

    const animate = () => {
      if (current < itemCount) {
        setVisibleItems(current + 1);
        current++;
        const timer = setTimeout(animate, staggerDelay);
        timers.push(timer);
      }
    };

    animate();

    return () => timers.forEach(clearTimeout);
  }, [itemCount, staggerDelay, reducedMotion]);

  const getItemStyles = (index: number): React.CSSProperties => {
    const isVisible = index < visibleItems;
    const delay = index * staggerDelay;

    let transform = 'none';
    if (!isVisible) {
      switch (direction) {
        case 'up':
          transform = 'translateY(20px)';
          break;
        case 'down':
          transform = 'translateY(-20px)';
          break;
        case 'left':
          transform = 'translateX(20px)';
          break;
        case 'right':
          transform = 'translateX(-20px)';
          break;
        case 'scale':
          transform = 'scale(0.95)';
          break;
      }
    }

    return {
      transform,
      opacity: isVisible ? 1 : 0,
      transition: reducedMotion
        ? 'none'
        : `transform ${duration}ms cubic-bezier(0.23, 1, 0.32, 1) ${delay}ms, opacity ${duration}ms ease ${delay}ms`,
    };
  };

  return { visibleItems, getItemStyles, isComplete: visibleItems >= itemCount };
};

// ============================================================================
// Spring Animation Hook
// ============================================================================

export interface SpringOptions {
  stiffness?: number;
  damping?: number;
  mass?: number;
}

export const useSpring = (target: number, options: SpringOptions = {}) => {
  const { stiffness = 100, damping = 10, mass = 1 } = options;
  const [current, setCurrent] = useState(target);
  const reducedMotion = useReducedMotion();
  const animationRef = useRef<number>();
  const velocityRef = useRef(0);
  const targetRef = useRef(target);

  useEffect(() => {
    targetRef.current = target;
  }, [target]);

  useEffect(() => {
    if (reducedMotion) {
      setCurrent(target);
      return;
    }

    const animate = () => {
      const displacement = targetRef.current - current;
      const springForce = displacement * stiffness;
      const dampingForce = velocityRef.current * damping;
      const acceleration = (springForce - dampingForce) / mass;

      velocityRef.current += acceleration * 0.016; // Assuming 60fps
      const newValue = current + velocityRef.current * 0.016;

      setCurrent(newValue);

      if (Math.abs(displacement) < 0.01 && Math.abs(velocityRef.current) < 0.01) {
        setCurrent(targetRef.current);
        velocityRef.current = 0;
        return;
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [target, stiffness, damping, mass, reducedMotion, current]);

  return current;
};

// ============================================================================
// Intersection Observer Hook for Scroll Animations
// ============================================================================

export interface ScrollRevealOptions {
  threshold?: number;
  rootMargin?: string;
  triggerOnce?: boolean;
}

export const useScrollReveal = (options: ScrollRevealOptions = {}) => {
  const { threshold = 0.1, rootMargin = '0px', triggerOnce = true } = options;
  const [isVisible, setIsVisible] = useState(false);
  const elementRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = elementRef.current;
    if (!element) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
          if (triggerOnce) {
            observer.unobserve(element);
          }
        } else if (!triggerOnce) {
          setIsVisible(false);
        }
      },
      { threshold, rootMargin }
    );

    observer.observe(element);

    return () => observer.disconnect();
  }, [threshold, rootMargin, triggerOnce]);

  return { ref: elementRef, isVisible };
};

// ============================================================================
// Ripple Effect Hook
// ============================================================================

export interface RippleState {
  x: number;
  y: number;
  size: number;
  id: number;
}

export const useRipple = () => {
  const [ripples, setRipples] = useState<RippleState[]>([]);
  const counterRef = useRef(0);
  const reducedMotion = useReducedMotion();

  const createRipple = useCallback((event: React.MouseEvent<HTMLElement>) => {
    if (reducedMotion) return;

    const element = event.currentTarget;
    const rect = element.getBoundingClientRect();

    const size = Math.max(rect.width, rect.height);
    const x = event.clientX - rect.left - size / 2;
    const y = event.clientY - rect.top - size / 2;

    const newRipple: RippleState = {
      x,
      y,
      size,
      id: counterRef.current++,
    };

    setRipples((prev) => [...prev, newRipple]);

    setTimeout(() => {
      setRipples((prev) => prev.filter((r) => r.id !== newRipple.id));
    }, 600);
  }, [reducedMotion]);

  return { ripples, createRipple };
};

// ============================================================================
// CSS Animation Classes
// ============================================================================

export const animationClasses = {
  // Fade animations
  fadeIn: 'animate-fade-in',
  fadeOut: 'animate-fade-out',

  // Slide animations
  slideInUp: 'animate-slide-in-up',
  slideInDown: 'animate-slide-in-down',
  slideInLeft: 'animate-slide-in-left',
  slideInRight: 'animate-slide-in-right',

  // Scale animations
  scaleIn: 'animate-scale-in',
  scaleOut: 'animate-scale-out',

  // Other
  spin: 'animate-spin',
  pulse: 'animate-pulse',
  bounce: 'animate-bounce',
  shimmer: 'animate-shimmer',
  blink: 'animate-blink',
};

// ============================================================================
// Duration Constants
// ============================================================================

export const DURATIONS = {
  instant: 50,
  fast: 100,
  normal: 200,
  slow: 300,
  slower: 400,
  slowest: 500,
} as const;

// ============================================================================
// Easing Functions
// ============================================================================

export const easings = {
  ease: 'cubic-bezier(0.4, 0, 0.2, 1)',
  easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
  easeOut: 'cubic-bezier(0, 0, 0.2, 1)',
  spring: 'cubic-bezier(0.34, 1.56, 0.64, 1)',
  smooth: 'cubic-bezier(0.23, 1, 0.32, 1)',
  snappy: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)',
} as const;

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Creates a transition CSS string
 */
export const createTransition = (
  properties: string[],
  duration: number = DURATIONS.normal,
  easing: string = easings.ease,
  delay: number = 0
): string => {
  return properties
    .map((prop) => `${prop} ${duration}ms ${easing} ${delay}ms`)
    .join(', ');
};

/**
 * Debounces a function
 */
export const debounce = <T extends (...args: unknown[]) => void>(
  fn: T,
  delay: number
): ((...args: Parameters<T>) => void) => {
  let timeoutId: NodeJS.Timeout;

  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  };
};

/**
 * Throttles a function
 */
export const throttle = <T extends (...args: unknown[]) => void>(
  fn: T,
  limit: number
): ((...args: Parameters<T>) => void) => {
  let inThrottle = false;

  return (...args: Parameters<T>) => {
    if (!inThrottle) {
      fn(...args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
};

export default {
  useReducedMotion,
  useFade,
  useSlide,
  useScale,
  useStagger,
  useSpring,
  useScrollReveal,
  useRipple,
  animationClasses,
  DURATIONS,
  easings,
  createTransition,
  debounce,
  throttle,
};
