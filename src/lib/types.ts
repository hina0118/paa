export type Email = {
  id: string;
  from: string;
  subject: string;
  preview: string;
  date: Date;
  read: boolean;
  starred: boolean;
  labels: string[];
  bodyPlain?: string;
  bodyHtml?: string;
};

export type DeliveryStatus =
  | 'not_shipped'
  | 'preparing'
  | 'shipped'
  | 'in_transit'
  | 'out_for_delivery'
  | 'delivered'
  | 'failed'
  | 'returned'
  | 'cancelled';

export type Order = {
  id: number;
  shopDomain?: string;
  orderNumber?: string;
  orderDate?: Date;
  createdAt: Date;
  updatedAt: Date;
};

export type Item = {
  id: number;
  orderId: number;
  itemName: string;
  itemNameNormalized?: string;
  price: number;
  quantity: number;
  category?: string;
  brand?: string;
  createdAt: Date;
  updatedAt: Date;
};

export type ItemImage = {
  id: number;
  itemId: number;
  fileName?: string;
  createdAt: Date;
};

export type Delivery = {
  id: number;
  orderId: number;
  trackingNumber?: string;
  carrier?: string;
  deliveryStatus: DeliveryStatus;
  estimatedDelivery?: Date;
  actualDelivery?: Date;
  lastCheckedAt?: Date;
  createdAt: Date;
  updatedAt: Date;
};

export type OrderWithDetails = Order & {
  items: (Item & { image?: ItemImage })[];
  deliveries: Delivery[];
};

export type Html = {
  id: number;
  url: string;
  htmlContent?: string;
  analysisStatus: 'pending' | 'completed';
  createdAt: Date;
  updatedAt: Date;
};

export type OrderEmail = {
  id: number;
  orderId: number;
  emailId: number;
  createdAt: Date;
};

export type OrderHtml = {
  id: number;
  orderId: number;
  htmlId: number;
  createdAt: Date;
};

export type OrderWithSources = Order & {
  emails: Email[];
  htmls: Html[];
  items: (Item & { image?: ItemImage })[];
  deliveries: Delivery[];
};

/** 商品一覧 1 件分（items + order + image + delivery + product_master） */
export type OrderItemRow = {
  id: number;
  orderId: number;
  itemName: string;
  itemNameNormalized: string | null;
  price: number;
  quantity: number;
  category: string | null;
  brand: string | null;
  createdAt: string;
  /** 表示用ショップ名（例: ホビーサーチ）。なければ shopDomain を表示 */
  shopName: string | null;
  shopDomain: string | null;
  orderNumber: string | null;
  orderDate: string | null;
  fileName: string | null;
  deliveryStatus: DeliveryStatus | null;
  /** product_master から取得（Gemini解析結果） */
  maker: string | null;
  series: string | null;
  productName: string | null;
  scale: string | null;
  isReissue: number | null;
};
