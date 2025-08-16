import { useEffect, RefObject } from 'react';

/**
 * Custom hook to handle click outside events for modals and dropdowns
 * @param ref - Reference to the element to detect clicks outside of
 * @param handler - Function to call when click outside is detected
 * @param enabled - Whether the click outside detection is enabled (default: true)
 */
export const useClickOutside = (
  ref: RefObject<HTMLElement>,
  handler: () => void,
  enabled: boolean = true
) => {
  useEffect(() => {
    if (!enabled) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        handler();
      }
    };

    // Add event listener to document
    document.addEventListener('mousedown', handleClickOutside);
    
    // Cleanup event listener on unmount
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [ref, handler, enabled]);
};
