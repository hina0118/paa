export type Email = {
  id: string;
  from: string;
  subject: string;
  preview: string;
  date: Date;
  read: boolean;
  starred: boolean;
  labels: string[];
};
