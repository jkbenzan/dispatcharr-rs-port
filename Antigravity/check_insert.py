import sqlite3
import traceback

conn = sqlite3.connect('dispatcharr.db')
c = conn.cursor()

try:
    c.execute("""
        INSERT INTO stream_sorting_rule (name, priority, property, operator, value, score_modifier)
        VALUES ('Test', 1, 'prop', '==', 'val', 10)
    """)
    conn.commit()
    print("SUCCESS INSERT")
    
    # Clean up
    c.execute("DELETE FROM stream_sorting_rule WHERE name='Test'")
    conn.commit()
except Exception as e:
    print("ERROR INSERT:")
    traceback.print_exc()
