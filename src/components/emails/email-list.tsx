import { emailData } from "@/lib/data";
import { columns } from "./columns";
import { DataTable } from "./data-table";
import { Inbox } from "lucide-react";

export function EmailList() {
  return (
    <div className="container mx-auto py-10 px-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <Inbox className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">受信トレイ</h1>
            <p className="text-sm text-muted-foreground mt-1">
              {emailData.length}件のメール
            </p>
          </div>
        </div>
      </div>
      <DataTable columns={columns} data={emailData} />
    </div>
  );
}
