import { createContext, useContext, useState, useCallback, type ReactNode } from "react";

interface ApplicationModalContextValue {
  isOpen: boolean;
  openModal: () => void;
  closeModal: () => void;
  setOpen: (open: boolean) => void;
}

const ApplicationModalContext = createContext<ApplicationModalContextValue | null>(null);

export function ApplicationModalProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);

  const openModal = useCallback(() => setIsOpen(true), []);
  const closeModal = useCallback(() => setIsOpen(false), []);
  const setOpen = useCallback((open: boolean) => setIsOpen(open), []);

  return (
    <ApplicationModalContext.Provider value={{ isOpen, openModal, closeModal, setOpen }}>
      {children}
    </ApplicationModalContext.Provider>
  );
}

export function useApplicationModal() {
  const context = useContext(ApplicationModalContext);
  return context ?? { isOpen: false, openModal: () => {}, closeModal: () => {}, setOpen: () => {} };
}
