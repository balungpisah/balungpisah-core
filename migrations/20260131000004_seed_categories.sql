-- Top-level categories
INSERT INTO categories (name, slug, description, icon, color, display_order) VALUES
('Infrastruktur', 'infrastruktur', 'Jalan, jembatan, bangunan publik', 'road', '#4CAF50', 1),
('Kesehatan', 'kesehatan', 'Layanan kesehatan, rumah sakit, puskesmas', 'hospital', '#F44336', 2),
('Pendidikan', 'pendidikan', 'Sekolah, guru, fasilitas pendidikan', 'school', '#2196F3', 3),
('Lingkungan', 'lingkungan', 'Sampah, polusi, ruang hijau', 'tree', '#8BC34A', 4),
('Keamanan', 'keamanan', 'Keamanan publik, kriminalitas', 'shield', '#FF9800', 5),
('Ekonomi', 'ekonomi', 'Pekerjaan, bisnis, kesejahteraan', 'briefcase', '#9C27B0', 6),
('Sosial', 'sosial', 'Bantuan sosial, layanan masyarakat', 'people', '#00BCD4', 7),
('Lainnya', 'lainnya', 'Masalah lain yang tidak masuk kategori', 'other', '#9E9E9E', 99);

-- Sub-categories for Infrastruktur
INSERT INTO categories (parent_id, name, slug, description, icon, color, display_order)
SELECT id, 'Jalan Rusak', 'jalan-rusak', 'Jalan berlubang, aspal rusak', 'road-damage', '#4CAF50', 1
FROM categories WHERE slug = 'infrastruktur';

INSERT INTO categories (parent_id, name, slug, description, icon, color, display_order)
SELECT id, 'Drainase', 'drainase', 'Saluran air tersumbat, banjir', 'water', '#4CAF50', 2
FROM categories WHERE slug = 'infrastruktur';

INSERT INTO categories (parent_id, name, slug, description, icon, color, display_order)
SELECT id, 'Listrik', 'listrik', 'Pemadaman, lampu jalan mati', 'bolt', '#4CAF50', 3
FROM categories WHERE slug = 'infrastruktur';

INSERT INTO categories (parent_id, name, slug, description, icon, color, display_order)
SELECT id, 'Air Bersih', 'air-bersih', 'PDAM, sumur, ketersediaan air', 'droplet', '#4CAF50', 4
FROM categories WHERE slug = 'infrastruktur';
