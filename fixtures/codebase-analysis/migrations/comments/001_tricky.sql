-- DROP VIEW this_is_a_comment — should NOT be flagged
/*
   DROP VIEW also_in_block_comment — should NOT be flagged
*/
CREATE TABLE foo (id int);
