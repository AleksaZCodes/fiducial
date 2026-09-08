"use client";

import * as React from "react";
import { Dialog as BaseDialog } from "@base-ui-components/react/dialog";

export interface DialogProps {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  children: React.ReactNode;
}

export function Dialog({ open, onOpenChange, children }: DialogProps) {
  return (
    <BaseDialog.Root open={open} onOpenChange={onOpenChange}>
      {children}
    </BaseDialog.Root>
  );
}

export function DialogTrigger({ children }: { children: React.ReactNode }) {
  return <BaseDialog.Trigger render={<>{children}</>} />;
}

export function DialogContent({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <BaseDialog.Portal>
      <BaseDialog.Backdrop className="fid-dialog-backdrop" />
      <BaseDialog.Popup className={["fid-dialog", className].filter(Boolean).join(" ")}>
        {children}
      </BaseDialog.Popup>
    </BaseDialog.Portal>
  );
}

export function DialogHeader({ children }: { children: React.ReactNode }) {
  return <div className="fid-dialog-header">{children}</div>;
}

export function DialogTitle({ children }: { children: React.ReactNode }) {
  return <BaseDialog.Title className="fid-dialog-title">{children}</BaseDialog.Title>;
}

export function DialogDescription({ children }: { children: React.ReactNode }) {
  return (
    <BaseDialog.Description className="fid-dialog-description">
      {children}
    </BaseDialog.Description>
  );
}

export function DialogClose({ children }: { children: React.ReactNode }) {
  return <BaseDialog.Close render={<>{children}</>} />;
}

export function DialogFooter({ children }: { children: React.ReactNode }) {
  return <div className="fid-dialog-footer">{children}</div>;
}
