CREATE TABLE timeslots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),  -- Auto-generate ID
    datetime TIMESTAMPTZ NOT NULL,
    available BOOLEAN NOT NULL DEFAULT true,
    booker_name VARCHAR NOT NULL DEFAULT '',  
    notes VARCHAR NOT NULL
);

CREATE OR REPLACE FUNCTION check_timeslot_availability()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.available = false THEN
        RAISE EXCEPTION 'Timeslot not available.';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER prevent_update_on_unavailable_timeslot
BEFORE UPDATE ON timeslots
FOR EACH ROW
EXECUTE FUNCTION check_timeslot_availability();
