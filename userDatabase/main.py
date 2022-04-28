import sqlite3


def create_db():
    try:
        conn = sqlite3.connect("credentials.db")
        c = conn.cursor()
        c.execute(''' CREATE TABLE credentials 
        (
        username TEXT PRIMARY KEY,
        password TEXT  
        )
        ''')
        conn.commit()
        return True
    except BaseException:
        return False
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()


def add_user(username, passwd):
    data_to_insert = [(username, hash_passwd(passwd))]
    try:
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        c.executemany("INSERT INTO credentials VALUES (? ,?)", data_to_insert)
        conn.commit()
    except sqlite3.IntegrityError:
        print("Error. Tried to add duplicate record")
    else:
        print("Success")
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()


def get_username(username):
    try:
        user = sanitize(username)
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        query = """SELECT * FROM credentials where username = ?"""
        c.execute(query, (user,))
        records = c.fetchone()
        names_to_return = records[0]
    except sqlite3.DatabaseError:
        print("Error. Could not retrieve data")
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return names_to_return


def get_passwd(username):
    try:
        user = sanitize(username)
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        query = """SELECT * FROM credentials where username = ?"""
        c.execute(query, (user,))
        records = c.fetchone()
        passwd = records[1]
    except sqlite3.DatabaseError:
        print("Error. Could not retrieve data")
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return passwd


def is_user_in_system(username):
    try:
        user = sanitize(username)
        is_in = False
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        for row in c.execute("SELECT * FROM credentials"):
            if user == row[0]:
                is_in = True
    except sqlite3.DatabaseError:
        print("Error. Could not retrieve data")
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return is_in


def authenticate(hashed_pw, plaintext):
    pass


def hash_passwd(plaintext):
    pass


def sanitize(in_word):
    pass


if __name__ == '__main__':
    print("Test Test")
