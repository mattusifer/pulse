ALTER TABLE tweets
ALTER COLUMN group_name TYPE TEXT[] USING ARRAY[group_name];
