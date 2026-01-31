-- Ticket status workflow
CREATE TYPE ticket_status AS ENUM (
    'submitted',       -- Agent created ticket (ready for processing)
    'processing',      -- Background job is extracting data
    'completed',       -- Successfully processed
    'failed'           -- Processing failed
);
