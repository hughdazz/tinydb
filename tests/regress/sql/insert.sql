create table t(a int, b int, c int);
insert into t(a, b, c) values(10, 20, 30);
insert into t(a, c) values(40, 50);
insert into t(b) values(60);
insert into t values (42, 62, 82);
select * from t;

create table t2(a int, b varchar, c int);
insert into t2(a, b, c) values(1, 'abc', 2);
insert into t2(b) values('def');
insert into t2(a) values(3);
insert into t2(c) values(4);
insert into t2(b, c, a) values('inverse column order', 70, 42);
select * from t2;

create table t3(a boolean, b boolean);
insert into t3(a, b) values (true, false);
select * from t3;


-- Test the projection behaviour specifing the columns


select b from t2;
select b, a from t2;
select c, * from t2;
select a, c from t2;
